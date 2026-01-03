# DynamoDB Checkpointer

AWS-native checkpointer for serverless and global deployments.

## Overview

The `DynamoDbCheckpointer`:
- Serverless, auto-scaling
- Global tables for multi-region
- Pay-per-request pricing
- AWS IAM integration

## Installation

```toml
[dependencies]
agents-sdk = { version = "0.0.29", features = ["dynamodb"] }
```

## Quick Start

```rust
use agents_sdk::{ConfigurableAgentBuilder, DynamoDbCheckpointer};
use std::sync::Arc;

let checkpointer = Arc::new(
    DynamoDbCheckpointer::new("agent-checkpoints").await?
);

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_checkpointer(checkpointer)
    .build()?;
```

## Table Setup

### Create Table (AWS CLI)

```bash
aws dynamodb create-table \
    --table-name agent-checkpoints \
    --attribute-definitions \
        AttributeName=thread_id,AttributeType=S \
    --key-schema \
        AttributeName=thread_id,KeyType=HASH \
    --billing-mode PAY_PER_REQUEST
```

### Create Table (Terraform)

```hcl
resource "aws_dynamodb_table" "agent_checkpoints" {
  name         = "agent-checkpoints"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "thread_id"

  attribute {
    name = "thread_id"
    type = "S"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }

  tags = {
    Environment = "production"
  }
}
```

## Configuration

### Basic

```rust
DynamoDbCheckpointer::new("agent-checkpoints").await?
```

### With Region

```rust
DynamoDbCheckpointer::new("agent-checkpoints")
    .await?
    .with_region("us-west-2")
```

### With TTL

```rust
use std::time::Duration;

DynamoDbCheckpointer::new("agent-checkpoints")
    .await?
    .with_ttl(Duration::from_secs(86400 * 30))  // 30 days
```

### With Custom Endpoint (LocalStack)

```rust
DynamoDbCheckpointer::new("agent-checkpoints")
    .await?
    .with_endpoint("http://localhost:4566")
```

## AWS Credentials

The checkpointer uses the standard AWS credential chain:

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. AWS credentials file
3. IAM role (EC2, ECS, Lambda)

### Environment Variables

```bash
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-east-1"
```

### IAM Policy

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
        }
    ]
}
```

## Item Structure

```json
{
    "thread_id": {"S": "user-123"},
    "state": {"S": "{\"messages\": [...]}"},
    "updated_at": {"S": "2024-01-01T12:00:00Z"},
    "ttl": {"N": "1735689600"}
}
```

## Operations

### Save State

```rust
agent.save_state("user-123").await?;
```

### Load State

```rust
agent.load_state("user-123").await?;
```

### Delete State

```rust
checkpointer.delete(&"user-123".into()).await?;
```

## Global Tables

For multi-region deployments:

```bash
aws dynamodb create-global-table \
    --global-table-name agent-checkpoints \
    --replication-group \
        RegionName=us-east-1 \
        RegionName=eu-west-1 \
        RegionName=ap-northeast-1
```

## Characteristics

| Property | Value |
|----------|-------|
| Latency | ~10ms |
| Persistence | Durable |
| Scalability | Global, unlimited |
| Dependencies | AWS DynamoDB |

## Best Practices

### 1. Enable TTL

```rust
// Automatic cleanup via DynamoDB TTL
.with_ttl(Duration::from_secs(86400 * 30))
```

### 2. Use On-Demand Capacity

For variable workloads, PAY_PER_REQUEST is recommended.

### 3. Monitor Costs

```bash
aws cloudwatch get-metric-statistics \
    --namespace AWS/DynamoDB \
    --metric-name ConsumedReadCapacityUnits \
    --dimensions Name=TableName,Value=agent-checkpoints \
    --start-time ... --end-time ... \
    --period 3600 \
    --statistics Sum
```

### 4. Use Global Tables for Multi-Region

Enable global tables for low-latency access worldwide.

## Lambda Example

```rust
use aws_lambda_events::event::apigw::ApiGatewayProxyRequest;
use lambda_runtime::{service_fn, LambdaEvent};
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiChatModel,
    OpenAiConfig,
    DynamoDbCheckpointer,
    state::AgentStateSnapshot,
};
use std::sync::Arc;

async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let checkpointer = Arc::new(
        DynamoDbCheckpointer::new("agent-checkpoints").await?
    );
    
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(std::env::var("OPENAI_API_KEY")?, "gpt-4o-mini")
    )?);
    
    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
        .with_model(model)
        .with_checkpointer(checkpointer)
        .build()?;
    
    let body: ChatRequest = serde_json::from_str(&event.payload.body.unwrap())?;
    
    // Resume conversation
    agent.load_state(&body.thread_id).await.ok();
    
    let response = agent.handle_message(
        &body.message,
        Arc::new(AgentStateSnapshot::default())
    ).await?;
    
    // Save state
    agent.save_state(&body.thread_id).await?;
    
    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        body: Some(serde_json::to_string(&response)?),
        ..Default::default()
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(service_fn(handler)).await
}
```

## LocalStack Testing

```rust
#[tokio::test]
async fn test_dynamodb_checkpointer() {
    let checkpointer = DynamoDbCheckpointer::new("test-table")
        .await
        .unwrap()
        .with_endpoint("http://localhost:4566");
    
    // Run tests against LocalStack
}
```

