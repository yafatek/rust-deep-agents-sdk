//! Deep Agent HTTP Server
//!
//! A production-ready web service that exposes Deep Agent functionality via REST API.
//!
//! Features:
//! - 🌐 HTTP REST API for agent interactions
//! - 🔄 Persistent sessions with unique IDs
//! - 🤖 Multiple specialized subagents (research, critique)
//! - 📁 File system operations
//! - 🔍 Real-time web search via Tavily
//! - 📊 Health checks and monitoring
//! - 🚀 Production-ready with proper error handling
//!
//! API Endpoints:
//! - POST /api/v1/chat - Send message to agent
//! - GET /api/v1/sessions/{id} - Get session info
//! - GET /api/v1/sessions - List all sessions
//! - GET /api/v1/health - Health check
//! - GET /api/v1/agents - List available agents and tools

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

use agents_core::persistence::InMemoryCheckpointer;
use agents_core::state::AgentStateSnapshot;
use agents_runtime::agent::SubAgentConfig;
use agents_runtime::providers::OpenAiConfig;
use agents_runtime::ConfigurableAgentBuilder;
use agents_toolkit::create_tool;

#[derive(Parser)]
#[command(name = "deep-agent-server")]
#[command(about = "HTTP server for Deep Agent interactions")]
struct Cli {
    /// Port to bind the server to
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Host to bind the server to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

// API Request/Response Types
#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    agent_type: Option<String>, // "research", "general", etc.
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    response: String,
    session_id: String,
    timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files_created: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools_used: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
struct SessionInfo {
    id: String,
    created_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    message_count: u32,
    agent_type: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_seconds: u64,
    active_sessions: usize,
}

#[derive(Debug, Serialize)]
struct AgentInfo {
    name: String,
    description: String,
    tools: Vec<String>,
    subagents: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AgentStatus {
    session_id: String,
    current_task: Option<String>,
    status: String, // "idle", "thinking", "calling_tool", "delegating"
    todos: Vec<TodoItem>,
    recent_actions: Vec<AgentAction>,
    active_subagent: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct TodoItem {
    id: String,
    content: String,
    status: String,   // "pending", "in_progress", "completed"
    priority: String, // "high", "medium", "low"
}

#[derive(Debug, Clone, Serialize)]
struct AgentAction {
    timestamp: DateTime<Utc>,
    action_type: String, // "tool_call", "delegation", "response"
    description: String,
    details: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
    timestamp: DateTime<Utc>,
}

// Application State
#[derive(Clone)]
struct AppState {
    agents: HashMap<String, Arc<dyn agents_core::agent::AgentHandle>>,
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    agent_status: Arc<RwLock<HashMap<String, AgentStatus>>>,
    start_time: DateTime<Utc>,
}

// Tavily API integration (same as before)
#[derive(Serialize)]
struct TavilyRequest {
    api_key: String,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_raw_content: Option<bool>,
}

#[derive(Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

#[derive(Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    content: String,
    #[serde(default)]
    score: f64,
}

async fn call_tavily_search(query: &str, max_results: Option<u32>) -> anyhow::Result<String> {
    let api_key = std::env::var("TAVILY_API_KEY")
        .map_err(|_| anyhow::anyhow!("TAVILY_API_KEY environment variable is required"))?;

    let client = reqwest::Client::new();
    let request = TavilyRequest {
        api_key,
        query: query.to_string(),
        max_results,
        include_raw_content: Some(true),
    };

    let response = client
        .post("https://api.tavily.com/search")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Tavily API error: {}", response.status()));
    }

    let tavily_response: TavilyResponse = response.json().await?;

    let mut formatted_results = String::new();
    formatted_results.push_str(&format!("# Search Results for: '{}'\n\n", query));

    for (i, result) in tavily_response.results.iter().enumerate() {
        formatted_results.push_str(&format!(
            "## Source {}: {}\n**URL:** {}\n**Relevance:** {:.2}\n\n**Content:**\n{}\n\n---\n\n",
            i + 1,
            result.title,
            result.url,
            result.score,
            result.content.chars().take(300).collect::<String>() + "..."
        ));
    }

    if tavily_response.results.is_empty() {
        formatted_results.push_str("No results found for this query.\n");
    }

    Ok(formatted_results)
}

// API Handlers
async fn chat_handler(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session_id = request
        .session_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let agent_type = request.agent_type.as_deref().unwrap_or("research");

    // Get or create session
    {
        let mut sessions = state.sessions.write().await;
        sessions
            .entry(session_id.clone())
            .or_insert_with(|| SessionInfo {
                id: session_id.clone(),
                created_at: Utc::now(),
                last_activity: Utc::now(),
                message_count: 0,
                agent_type: agent_type.to_string(),
            });

        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_activity = Utc::now();
            session.message_count += 1;
        }
    }

    // Get the appropriate agent
    let agent = state.agents.get(agent_type).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Unknown agent type: {}", agent_type),
                code: "INVALID_AGENT_TYPE".to_string(),
                timestamp: Utc::now(),
            }),
        )
    })?;

    // Update agent status to "thinking"
    {
        let mut status_map = state.agent_status.write().await;
        status_map.insert(
            session_id.clone(),
            AgentStatus {
                session_id: session_id.clone(),
                current_task: Some(request.message.clone()),
                status: "thinking".to_string(),
                todos: vec![], // TODO: Extract from agent state
                recent_actions: vec![AgentAction {
                    timestamp: Utc::now(),
                    action_type: "user_message".to_string(),
                    description: "Received user message".to_string(),
                    details: Some(request.message.clone()),
                }],
                active_subagent: None,
            },
        );
    }

    // Process the message
    let user_message = agents_core::messaging::AgentMessage {
        role: agents_core::messaging::MessageRole::User,
        content: agents_core::messaging::MessageContent::Text(request.message.clone()),
        metadata: None,
    };

    match agent
        .handle_message(user_message, Arc::new(AgentStateSnapshot::default()))
        .await
    {
        Ok(response) => {
            let response_text = response
                .content
                .as_text()
                .unwrap_or("No response")
                .to_string();

            // Update agent status to "idle"
            {
                let mut status_map = state.agent_status.write().await;
                if let Some(status) = status_map.get_mut(&session_id) {
                    status.status = "idle".to_string();
                    status.current_task = None;
                    status.recent_actions.push(AgentAction {
                        timestamp: Utc::now(),
                        action_type: "response".to_string(),
                        description: "Provided response to user".to_string(),
                        details: Some(response_text.chars().take(100).collect::<String>() + "..."),
                    });
                }
            }

            Ok(Json(ChatResponse {
                response: response_text,
                session_id,
                timestamp: Utc::now(),
                files_created: None, // TODO: Track file operations
                tools_used: None,    // TODO: Track tool usage
            }))
        }
        Err(e) => {
            tracing::error!("Agent processing failed: {:?}", e);

            // Update agent status to error
            {
                let mut status_map = state.agent_status.write().await;
                if let Some(status) = status_map.get_mut(&session_id) {
                    status.status = "error".to_string();
                    status.current_task = None;
                    status.recent_actions.push(AgentAction {
                        timestamp: Utc::now(),
                        action_type: "error".to_string(),
                        description: "Agent processing failed".to_string(),
                        details: Some(format!("{}", e)),
                    });
                }
            }

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Agent processing failed: {}", e),
                    code: "AGENT_ERROR".to_string(),
                    timestamp: Utc::now(),
                }),
            ))
        }
    }
}

async fn get_session_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionInfo>, (StatusCode, Json<ErrorResponse>)> {
    let sessions = state.sessions.read().await;

    match sessions.get(&session_id) {
        Some(session) => Ok(Json(session.clone())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Session not found".to_string(),
                code: "SESSION_NOT_FOUND".to_string(),
                timestamp: Utc::now(),
            }),
        )),
    }
}

async fn list_sessions_handler(State(state): State<AppState>) -> Json<Vec<SessionInfo>> {
    let sessions = state.sessions.read().await;
    Json(sessions.values().cloned().collect())
}

async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let uptime = Utc::now().signed_duration_since(state.start_time);
    let sessions = state.sessions.read().await;

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime.num_seconds() as u64,
        active_sessions: sessions.len(),
    })
}

async fn agents_info_handler(State(_state): State<AppState>) -> Json<Vec<AgentInfo>> {
    let agents = vec![AgentInfo {
        name: "research".to_string(),
        description: "Deep research agent with specialized subagents for comprehensive analysis"
            .to_string(),
        tools: vec![
            "write_file".to_string(),
            "read_file".to_string(),
            "edit_file".to_string(),
            "ls".to_string(),
            "write_todos".to_string(),
            "task".to_string(),
        ],
        subagents: vec!["research-agent".to_string(), "critique-agent".to_string()],
    }];

    Json(agents)
}

async fn get_agent_status_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<AgentStatus>, (StatusCode, Json<ErrorResponse>)> {
    let status_map = state.agent_status.read().await;

    match status_map.get(&session_id) {
        Some(status) => Ok(Json(status.clone())),
        None => {
            // Create default status for new sessions
            let default_status = AgentStatus {
                session_id: session_id.clone(),
                current_task: None,
                status: "idle".to_string(),
                todos: vec![],
                recent_actions: vec![],
                active_subagent: None,
            };
            Ok(Json(default_status))
        }
    }
}

async fn list_agent_status_handler(State(state): State<AppState>) -> Json<Vec<AgentStatus>> {
    let status_map = state.agent_status.read().await;
    Json(status_map.values().cloned().collect())
}

async fn create_research_agent() -> anyhow::Result<Arc<dyn agents_core::agent::AgentHandle>> {
    // Create internet search tool
    let internet_search = create_tool(
        "internet_search",
        "Search the internet for information using Tavily API",
        |args: serde_json::Value| async move {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("default query");

            let max_results = args
                .get("max_results")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32)
                .unwrap_or(5);

            match call_tavily_search(query, Some(max_results)).await {
                Ok(results) => Ok(results),
                Err(e) => Ok(format!("❌ Search failed: {}", e)),
            }
        },
    );

    // Create specialized subagents
    let research_subagent = SubAgentConfig::new(
        "research-agent",
        "Researcher with internet search",
        r#"You are a researcher. For any research task, first search the internet then provide a detailed answer.

To search: {"tool_calls": [{"name": "internet_search", "args": {"query": "search terms", "max_results": 5}}]}

Always search first, then provide comprehensive answers based on the results."#
    )
    .with_tools(vec![internet_search.clone()]);

    let critique_subagent = SubAgentConfig::new(
        "critique-agent",
        "Expert editor for critiquing and improving content",
        r#"You are a dedicated editor. Provide detailed feedback on content quality.

Check for: structure, comprehensiveness, clarity, accuracy, and proper citations.

If you need additional information for better critique, use internet_search:
```json
{"tool_calls": [{"name": "internet_search", "args": {"query": "your query", "max_results": 3}}]}
```

Provide actionable feedback to improve content quality."#
    )
    .with_tools(vec![internet_search.clone()]);

    // Main agent instructions - very explicit
    let main_instructions = r#"You MUST respond with JSON tool calls only. Never give text responses.

For research: {"tool_calls": [{"name": "task", "args": {"description": "research TOPIC", "subagent_type": "research-agent"}}]}
For other: {"tool_calls": []}

Examples:
"Research AI" -> {"tool_calls": [{"name": "task", "args": {"description": "research AI", "subagent_type": "research-agent"}}]}
"Hello" -> {"tool_calls": []}"#;

    // Create OpenAI configuration
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable is required"))?;

    println!(
        "🔑 Using OpenAI API key: {}...{}",
        &api_key[..std::cmp::min(8, api_key.len())],
        &api_key[api_key.len().saturating_sub(4)..]
    );

    let openai_config = OpenAiConfig::new(api_key, "gpt-4o-mini");

    // Create checkpointer
    let checkpointer = Arc::new(InMemoryCheckpointer::new());

    // Build the agent
    let agent = ConfigurableAgentBuilder::new(main_instructions)
        .with_openai_chat(openai_config)?
        .with_builtin_tools(["ls", "read_file", "write_file", "edit_file", "write_todos"]) // Add filesystem tools
        .with_subagent_config([research_subagent, critique_subagent]) // Subagents have internet_search
        .with_checkpointer(checkpointer)
        .build()?;

    Ok(Arc::new(agent))
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_env_filter("deep_agent_server=debug,agents_runtime=debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    dotenv::dotenv().ok();

    println!("🚀 Deep Agent HTTP Server");
    println!("========================");

    // Create agents
    println!("🤖 Initializing Deep Research Agent...");
    let research_agent = create_research_agent().await?;

    let mut agents = HashMap::new();
    agents.insert("research".to_string(), research_agent);

    // Create application state
    let state = AppState {
        agents,
        sessions: Arc::new(RwLock::new(HashMap::new())),
        agent_status: Arc::new(RwLock::new(HashMap::new())),
        start_time: Utc::now(),
    };

    // Build the router
    let app = Router::new()
        .route("/api/v1/chat", post(chat_handler))
        .route("/api/v1/sessions/:id", get(get_session_handler))
        .route("/api/v1/sessions", get(list_sessions_handler))
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/agents", get(agents_info_handler))
        .route("/api/v1/status/:session_id", get(get_agent_status_handler))
        .route("/api/v1/status", get(list_agent_status_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let bind_addr = format!("{}:{}", cli.host, cli.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    println!("✅ Server ready!");
    println!("🌐 Listening on: http://{}", bind_addr);
    println!("📚 API Documentation:");
    println!("   POST /api/v1/chat           - Send message to agent");
    println!("   GET  /api/v1/sessions/:id   - Get session info");
    println!("   GET  /api/v1/sessions       - List all sessions");
    println!("   GET  /api/v1/health         - Health check");
    println!("   GET  /api/v1/agents         - List available agents");
    println!("\n🔧 Example usage:");
    println!("   curl -X POST http://{}/api/v1/chat \\", bind_addr);
    println!("     -H 'Content-Type: application/json' \\");
    println!("     -d '{{\"message\": \"What is quantum computing?\"}}'");
    println!("\n⏹️  Press Ctrl+C to stop the server");

    // Start the server
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("👋 Server shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\n🛑 Shutdown signal received, stopping server...");
}
