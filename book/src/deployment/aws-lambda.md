# AWS Lambda Deployment

Deploy agents as serverless functions.

## Prerequisites

- AWS CLI configured
- Cargo Lambda installed: `cargo install cargo-lambda`
- OpenAI/Anthropic API key in AWS Secrets Manager

## Project Setup

```bash
cargo lambda new agent-lambda
cd agent-lambda
```

## Cargo.toml

```toml
[package]
name = "agent-lambda"
version = "0.1.0"
edition = "2021"

[dependencies]
agents-sdk = { version = "0.0.29", features = ["dynamodb"] }
aws_lambda_events = "0.12"
lambda_runtime = "0.9"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Handler Code

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    DynamoDbCheckpointer,
    state::AgentStateSnapshot,
};
use aws_lambda_events::event::apigw::{
    ApiGatewayProxyRequest,
    ApiGatewayProxyResponse,
};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    thread_id: String,
}

#[derive(Serialize)]
struct ChatResponse {
    response: String,
}

async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    // Parse request
    let body = event.payload.body.unwrap_or_default();
    let request: ChatRequest = serde_json::from_str(&body)?;
    
    // Get API key from environment (set from Secrets Manager)
    let api_key = std::env::var("OPENAI_API_KEY")?;
    
    // Create model
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);
    
    // Create checkpointer
    let checkpointer = Arc::new(
        DynamoDbCheckpointer::new("agent-checkpoints").await?
    );
    
    // Build agent
    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
        .with_model(model)
        .with_checkpointer(checkpointer.clone())
        .build()?;
    
    // Load state if exists
    agent.load_state(&request.thread_id).await.ok();
    
    // Handle message
    let response = agent.handle_message(
        &request.message,
        Arc::new(AgentStateSnapshot::default())
    ).await?;
    
    // Save state
    agent.save_state(&request.thread_id).await?;
    
    // Return response
    let chat_response = ChatResponse {
        response: response.content.as_text()
            .unwrap_or_default()
            .to_string(),
    };
    
    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: Default::default(),
        multi_value_headers: Default::default(),
        body: Some(serde_json::to_string(&chat_response)?),
        is_base64_encoded: false,
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(service_fn(handler)).await
}
```

## Build & Deploy

```bash
# Build for ARM64 (Graviton)
cargo lambda build --release --arm64

# Deploy
cargo lambda deploy agent-lambda \
    --iam-role arn:aws:iam::123456789:role/lambda-role
```

## Infrastructure (Terraform)

```hcl
resource "aws_lambda_function" "agent" {
  function_name = "agent-lambda"
  role          = aws_iam_role.lambda.arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["arm64"]
  timeout       = 30
  memory_size   = 256

  environment {
    variables = {
      OPENAI_API_KEY = data.aws_secretsmanager_secret_version.openai.secret_string
    }
  }
}

resource "aws_dynamodb_table" "checkpoints" {
  name         = "agent-checkpoints"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "thread_id"

  attribute {
    name = "thread_id"
    type = "S"
  }
}
```

## IAM Policy

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "dynamodb:GetItem",
                "dynamodb:PutItem",
                "dynamodb:DeleteItem"
            ],
            "Resource": "arn:aws:dynamodb:*:*:table/agent-checkpoints"
        },
        {
            "Effect": "Allow",
            "Action": "secretsmanager:GetSecretValue",
            "Resource": "arn:aws:secretsmanager:*:*:secret:openai-api-key*"
        }
    ]
}
```

## Best Practices

1. **Use Graviton (ARM64)** - Better price/performance
2. **DynamoDB for state** - Serverless-friendly
3. **Secrets Manager** - Secure API key storage
4. **Set reasonable timeout** - 30s for most operations
5. **Enable provisioned concurrency** - Reduce cold starts

