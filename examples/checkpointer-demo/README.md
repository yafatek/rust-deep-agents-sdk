# Checkpointer Demo

This example demonstrates how to use different persistence backends with the Rust Deep Agents SDK.

## Available Backends

- **InMemory** (default): No persistence, state lives in RAM only
- **Redis**: High-performance in-memory data store
- **PostgreSQL**: Relational database with ACID guarantees
- **DynamoDB**: AWS-managed NoSQL database

## Prerequisites

### Redis
```bash
# Using Docker
docker run -d -p 6379:6379 redis:7-alpine

# Or install locally
brew install redis  # macOS
sudo apt-get install redis-server  # Ubuntu
```

### PostgreSQL
```bash
# Using Docker
docker run -d \
  -e POSTGRES_DB=agents \
  -e POSTGRES_USER=user \
  -e POSTGRES_PASSWORD=pass \
  -p 5432:5432 \
  postgres:16-alpine

# Or install locally
brew install postgresql  # macOS
sudo apt-get install postgresql  # Ubuntu
```

### DynamoDB
```bash
# Using LocalStack (for local testing)
docker run -d -p 4566:4566 localstack/localstack

# Create table
aws dynamodb create-table \
  --endpoint-url http://localhost:4566 \
  --table-name agent-checkpoints \
  --attribute-definitions AttributeName=thread_id,AttributeType=S \
  --key-schema AttributeName=thread_id,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST

# Or use real AWS (requires credentials)
aws dynamodb create-table \
  --table-name agent-checkpoints \
  --attribute-definitions AttributeName=thread_id,AttributeType=S \
  --key-schema AttributeName=thread_id,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST
```

## Running the Examples

### InMemory (No Persistence)
```bash
cargo run --example checkpointer-demo
```

### Redis
```bash
# Default connection (localhost:6379)
cargo run --example checkpointer-demo --features redis -- --backend redis

# Custom connection
cargo run --example checkpointer-demo --features redis -- \
  --backend redis \
  --connection "redis://127.0.0.1:6379"
```

### PostgreSQL
```bash
# Default connection
cargo run --example checkpointer-demo --features postgres -- --backend postgres

# Custom connection
cargo run --example checkpointer-demo --features postgres -- \
  --backend postgres \
  --connection "postgresql://user:pass@localhost/agents"
```

### DynamoDB
```bash
# Using LocalStack
export AWS_ENDPOINT_URL=http://localhost:4566
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
cargo run --example checkpointer-demo --features dynamodb -- \
  --backend dynamodb \
  --connection "agent-checkpoints"

# Using real AWS (requires proper credentials)
cargo run --example checkpointer-demo --features dynamodb -- \
  --backend dynamodb \
  --connection "agent-checkpoints"
```

## Testing Persistence

Run the example twice with the same `--thread-id` to see state persistence in action:

```bash
# First run - creates new state
cargo run --example checkpointer-demo --features redis -- \
  --backend redis \
  --thread-id "user-123"

# Second run - loads previous state
cargo run --example checkpointer-demo --features redis -- \
  --backend redis \
  --thread-id "user-123"
```

## What It Demonstrates

1. **Checkpointer Creation**: How to create and configure different checkpointer backends
2. **State Persistence**: Saving and loading agent state across sessions
3. **Thread Management**: Working with multiple conversation threads
4. **Error Handling**: Graceful handling of connection errors
5. **Feature Flags**: Using Cargo features to include only needed backends

## Performance Comparison

Run each backend and observe the differences:

| Backend | Save Speed | Load Speed | Setup | Durability |
|---------|-----------|------------|-------|------------|
| InMemory | ⚡ Instant | ⚡ Instant | None | ❌ Lost on restart |
| Redis | ⚡ <5ms | ⚡ <5ms | Easy | ✅ With AOF/RDB |
| PostgreSQL | 🚀 ~10ms | 🚀 ~10ms | Medium | ✅ ACID |
| DynamoDB | 🚀 ~20ms | 🚀 ~20ms | Easy (AWS) | ✅ Durable |

## Troubleshooting

### Redis Connection Failed
```bash
# Check if Redis is running
redis-cli ping
# Should return "PONG"
```

### PostgreSQL Connection Failed
```bash
# Check if PostgreSQL is running
psql -U postgres -c "SELECT 1"

# Create database if needed
createdb agents
```

### DynamoDB Connection Failed
```bash
# Test LocalStack
curl http://localhost:4566/_localstack/health

# Test AWS credentials
aws sts get-caller-identity
```

## Next Steps

- Try modifying the agent's tools and instructions
- Implement custom state management logic
- Add more conversation threads
- Benchmark different backends for your use case

