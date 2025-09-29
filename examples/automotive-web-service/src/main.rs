use agents_sdk::{
    agent::AgentHandle,
    llm::StreamChunk,
    messaging::{AgentMessage, MessageContent, MessageRole},
    persistence::{Checkpointer, InMemoryCheckpointer},
    state::AgentStateSnapshot,
    tool, ConfigurableAgentBuilder, OpenAiChatModel, OpenAiConfig, SubAgentConfig,
    SummarizationConfig,
};
use axum::{
    extract::{Query, State as AxumState},
    http::Method,
    response::{
        sse::{Event, Sse},
        IntoResponse, Json,
    },
    routing::get,
    Router,
};
use futures::{stream::Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

// ============================================================================
// Data Structures (same as automotive example)
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct DiagnosticResult {
    issue_type: String,
    severity: String,
    recommended_service: String,
    estimated_cost_aed: f64,
    estimated_duration_hours: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SupportTicket {
    ticket_id: String,
    customer_name: String,
    vehicle_model: String,
    issue_description: String,
    priority: String,
    status: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Appointment {
    appointment_id: String,
    customer_name: String,
    vehicle_model: String,
    service_type: String,
    date: String,
    time: String,
    location: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PaymentLink {
    payment_id: String,
    amount_aed: f64,
    description: String,
    link: String,
    expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Notification {
    notification_id: String,
    recipient: String,
    channel: String,
    message: String,
    sent_at: String,
    status: String,
}

// ============================================================================
// Tools (same as automotive example)
// ============================================================================

#[tool("Diagnoses car issues based on symptoms and provides recommendations")]
fn diagnose_car_issue(
    symptoms: String,
    _vehicle_make: String,
    _vehicle_model: String,
    _year: i32,
    mileage_km: i32,
) -> String {
    let diagnostic = DiagnosticResult {
        issue_type: format!("Based on '{}', likely issue detected", symptoms),
        severity: if symptoms.to_lowercase().contains("noise") {
            "Medium".to_string()
        } else if symptoms.to_lowercase().contains("smoke") {
            "High".to_string()
        } else {
            "Low".to_string()
        },
        recommended_service: format!(
            "Recommended: Full inspection + {} service",
            if mileage_km > 100000 {
                "major"
            } else {
                "minor"
            }
        ),
        estimated_cost_aed: if symptoms.to_lowercase().contains("engine") {
            2500.0
        } else if symptoms.to_lowercase().contains("brake") {
            800.0
        } else {
            500.0
        },
        estimated_duration_hours: 2.5,
    };
    serde_json::to_string_pretty(&diagnostic).unwrap()
}

#[tool("Calculates service cost with UAE-specific pricing")]
fn calculate_service_cost(
    service_type: String,
    vehicle_make: String,
    _vehicle_model: String,
    year: i32,
) -> String {
    let base_price = match service_type.to_lowercase().as_str() {
        "oil_change" => 150.0,
        "brake_service" => 800.0,
        "ac_service" => 600.0,
        "major_service" => 2500.0,
        _ => 500.0,
    };

    let brand_multiplier = match vehicle_make.to_lowercase().as_str() {
        "mercedes" | "bmw" | "audi" => 1.5,
        "toyota" | "honda" | "nissan" => 1.0,
        _ => 1.2,
    };

    let age_factor = if year < 2015 { 1.2 } else { 1.0 };

    let subtotal = base_price * brand_multiplier * age_factor;
    let vat = subtotal * 0.05;
    let total = subtotal + vat;

    format!(
        "{{\"subtotal_aed\": {:.2}, \"vat_aed\": {:.2}, \"total_aed\": {:.2}}}",
        subtotal, vat, total
    )
}

#[tool("Creates a support ticket for tracking customer issues")]
fn create_support_ticket(
    customer_name: String,
    vehicle_model: String,
    issue_description: String,
    priority: String,
) -> String {
    let ticket = SupportTicket {
        ticket_id: format!("TKT-{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
        customer_name,
        vehicle_model,
        issue_description,
        priority,
        status: "Open".to_string(),
        created_at: chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
    };
    serde_json::to_string_pretty(&ticket).unwrap()
}

#[tool("Checks ticket status by ID")]
fn check_ticket_status(ticket_id: String) -> String {
    format!(
        "{{\"ticket_id\": \"{}\", \"status\": \"In Progress\", \"last_updated\": \"{}\"}}",
        ticket_id,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    )
}

#[tool("Checks service center availability")]
fn check_availability(location: String, preferred_date: String, service_type: String) -> String {
    let available_slots = vec!["09:00 AM", "11:00 AM", "02:00 PM", "04:00 PM"];
    format!(
        "{{\"location\": \"{}\", \"date\": \"{}\", \"service_type\": \"{}\", \"available_slots\": {:?}}}",
        location, preferred_date, service_type, available_slots
    )
}

#[tool("Books an appointment at the service center")]
fn book_appointment(
    customer_name: String,
    vehicle_model: String,
    service_type: String,
    date: String,
    time: String,
    location: String,
) -> String {
    let appointment = Appointment {
        appointment_id: format!("APT-{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
        customer_name,
        vehicle_model,
        service_type,
        date,
        time,
        location,
    };
    serde_json::to_string_pretty(&appointment).unwrap()
}

#[tool("Generates a secure payment link")]
fn generate_payment_link(amount_aed: f64, description: String) -> String {
    let payment = PaymentLink {
        payment_id: format!("PAY-{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
        amount_aed,
        description,
        link: format!("https://pay.example.ae/{}", Uuid::new_v4()),
        expires_at: chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
    };
    serde_json::to_string_pretty(&payment).unwrap()
}

#[tool("Confirms payment status")]
fn confirm_payment(payment_id: String) -> String {
    format!(
        "{{\"payment_id\": \"{}\", \"status\": \"Completed\", \"confirmed_at\": \"{}\"}}",
        payment_id,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    )
}

#[tool("Sends notification via SMS, Email, or WhatsApp")]
fn send_notification(recipient: String, channel: String, message: String) -> String {
    let notification = Notification {
        notification_id: format!("NOT-{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
        recipient,
        channel,
        message,
        sent_at: chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
        status: "Sent".to_string(),
    };
    serde_json::to_string_pretty(&notification).unwrap()
}

#[tool("Collects customer feedback and satisfaction rating")]
fn collect_feedback(
    _customer_name: String,
    rating: i32,
    comments: String,
    service_date: String,
) -> String {
    format!(
        "{{\"rating\": {}, \"comments\": \"{}\", \"service_date\": \"{}\", \"recorded_at\": \"{}\"}}",
        rating,
        comments,
        service_date,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    )
}

#[tool("Analyzes feedback trends for service improvement")]
fn analyze_feedback_trends(period: String) -> String {
    format!(
        "{{\"period\": \"{}\", \"average_rating\": 4.5, \"total_responses\": 150, \"satisfaction_rate\": \"90%\"}}",
        period
    )
}

// ============================================================================
// Web Server Types
// ============================================================================

#[derive(Clone)]
struct AppState {
    agent: Arc<dyn AgentHandle>,
    _checkpointer: Arc<dyn Checkpointer>,
    sessions: Arc<RwLock<HashMap<String, Vec<AgentMessage>>>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    session_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct ChatResponse {
    session_id: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct SessionInfo {
    session_id: String,
    message_count: usize,
    created_at: String,
}

// ============================================================================
// Agent Setup
// ============================================================================

fn create_automotive_agent(
    checkpointer: Arc<dyn Checkpointer>,
) -> anyhow::Result<Arc<dyn AgentHandle>> {
    // Create OpenAI model
    let openai_config = OpenAiConfig {
        api_key: std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY environment variable is required"),
        model: "gpt-4o-mini".to_string(),
        api_url: None,
    };
    let model = Arc::new(OpenAiChatModel::new(openai_config)?);

    // Create sub-agents
    let diagnostic_agent = SubAgentConfig::new(
        "diagnostic-agent",
        "Expert automotive diagnostic specialist for UAE vehicles",
        "You are an expert automotive diagnostic specialist with 20+ years experience in UAE. \
         You understand all car makes and models sold in the UAE market.",
    )
    .with_tools(vec![
        DiagnoseCarIssueTool::as_tool(),
        CalculateServiceCostTool::as_tool(),
    ]);

    let booking_agent = SubAgentConfig::new(
        "booking-agent",
        "Appointment scheduling specialist for service centers",
        "You are a booking specialist for automotive service centers in UAE.",
    )
    .with_tools(vec![
        CheckAvailabilityTool::as_tool(),
        BookAppointmentTool::as_tool(),
    ]);

    let ticketing_agent = SubAgentConfig::new(
        "ticketing-agent",
        "Support ticket management specialist",
        "You are a support ticket specialist managing customer service requests.",
    )
    .with_tools(vec![
        CreateSupportTicketTool::as_tool(),
        CheckTicketStatusTool::as_tool(),
    ]);

    let payment_agent = SubAgentConfig::new(
        "payment-agent",
        "Payment processing specialist",
        "You are a payment specialist handling secure transactions in AED.",
    )
    .with_tools(vec![
        GeneratePaymentLinkTool::as_tool(),
        ConfirmPaymentTool::as_tool(),
    ]);

    let notification_agent = SubAgentConfig::new(
        "notification-agent",
        "Multi-channel communication specialist",
        "You are a notification specialist sending updates via SMS, Email, and WhatsApp.",
    )
    .with_tools(vec![SendNotificationTool::as_tool()]);

    let feedback_agent = SubAgentConfig::new(
        "feedback-agent",
        "Customer satisfaction specialist",
        "You are a feedback specialist collecting and analyzing customer satisfaction.",
    )
    .with_tools(vec![
        CollectFeedbackTool::as_tool(),
        AnalyzeFeedbackTrendsTool::as_tool(),
    ]);

    // Build main agent with checkpointing and summarization
    let agent = ConfigurableAgentBuilder::new(
        "You are the main customer service coordinator for a premium automotive maintenance service in UAE. \
         You are professional, friendly, and fluent in English and Arabic.\n\n\
         Your role is to provide seamless service from first contact to feedback collection.",
    )
    .with_model(model)
    .with_subagent_config([
        diagnostic_agent,
        booking_agent,
        ticketing_agent,
        payment_agent,
        notification_agent,
        feedback_agent,
    ])
    .with_checkpointer(checkpointer)
    .with_summarization(SummarizationConfig {
        messages_to_keep: 10,
        summary_note: "Previous conversation summary".to_string(),
    })
    .with_auto_general_purpose(true)
    .build()?;

    Ok(Arc::new(agent))
}

// ============================================================================
// HTTP Handlers
// ============================================================================

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "Automotive Web Service",
        "version": "0.1.0"
    }))
}

async fn chat_sse(
    AxumState(state): AxumState<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let message = params.get("message").cloned().unwrap_or_default();
    let session_id = params
        .get("session_id")
        .cloned()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let stream = async_stream::stream! {
        // Send session ID first
        yield Ok(Event::default()
            .event("session")
            .data(serde_json::json!({"session_id": session_id}).to_string()));

        // Create user message
        let user_message = AgentMessage {
            role: MessageRole::User,
            content: MessageContent::Text(message.clone()),
            metadata: None,
        };

        // Get agent stream
        match state
            .agent
            .handle_message_stream(user_message, Arc::new(AgentStateSnapshot::default()))
            .await
        {
            Ok(mut stream) => {
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(StreamChunk::TextDelta(delta)) => {
                            if !delta.is_empty() {
                                yield Ok(Event::default()
                                    .event("delta")
                                    .data(serde_json::json!({"text": delta}).to_string()));
                            }
                        }
                        Ok(StreamChunk::Done { message }) => {
                            tracing::info!("Received Done chunk, sending done event to client");
                            if let MessageContent::Text(text) = message.content {
                                tracing::debug!("Done event text length: {}", text.len());
                                yield Ok(Event::default()
                                    .event("done")
                                    .data(serde_json::json!({"text": text}).to_string()));
                            }
                            break;
                        }
                        Ok(StreamChunk::Error(error)) => {
                            tracing::error!("Stream error: {}", error);
                            yield Ok(Event::default()
                                .event("error")
                                .data(serde_json::json!({"error": error}).to_string()));
                            break;
                        }
                        Err(e) => {
                            tracing::error!("Stream processing error: {}", e);
                            yield Ok(Event::default()
                                .event("error")
                                .data(serde_json::json!({"error": e.to_string()}).to_string()));
                            break;
                        }
                    }
                }
                tracing::info!("Stream loop ended");
            }
            Err(e) => {
                yield Ok(Event::default()
                    .event("error")
                    .data(serde_json::json!({"error": e.to_string()}).to_string()));
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

async fn list_sessions(AxumState(state): AxumState<AppState>) -> impl IntoResponse {
    let sessions = state.sessions.read().await;
    let session_list: Vec<SessionInfo> = sessions
        .iter()
        .map(|(id, messages)| SessionInfo {
            session_id: id.clone(),
            message_count: messages.len(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
        .collect();
    Json(session_list)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    println!("🚗 UAE Automotive Web Service");
    println!("================================\n");

    // Create checkpointer
    let checkpointer = Arc::new(InMemoryCheckpointer::new());

    // Create agent
    println!("🔧 Building automotive agent...");
    let agent = create_automotive_agent(checkpointer.clone())?;
    println!("✅ Agent ready!\n");

    // Create app state
    let state = AppState {
        agent,
        _checkpointer: checkpointer,
        sessions: Arc::new(RwLock::new(HashMap::new())),
    };

    // Setup CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/chat/stream", get(chat_sse))
        .route("/sessions", get(list_sessions))
        .layer(cors)
        .with_state(state);

    // Start server
    let addr = "0.0.0.0:3001";
    println!("🌐 Server running on http://{}", addr);
    println!("📡 SSE endpoint: http://{}/chat/stream", addr);
    println!("❤️  Health check: http://{}/health\n", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
