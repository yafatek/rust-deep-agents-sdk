# Docker Deployment

Containerize your agent for consistent deployments.

## Dockerfile

```dockerfile
# Build stage
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source
COPY src ./src

# Build application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/my-agent /app/my-agent

# Non-root user
RUN useradd -r -s /bin/false agent
USER agent

EXPOSE 3000

CMD ["./my-agent"]
```

## Docker Compose

```yaml
version: '3.8'

services:
  agent:
    build: .
    ports:
      - "3000:3000"
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - REDIS_URL=redis://redis:6379
    depends_on:
      - redis
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data

volumes:
  redis_data:
```

## Build & Run

```bash
# Build
docker build -t my-agent .

# Run
docker run -p 3000:3000 \
    -e OPENAI_API_KEY=$OPENAI_API_KEY \
    my-agent
```

## Multi-Architecture

```dockerfile
# For both AMD64 and ARM64
FROM --platform=$BUILDPLATFORM rust:1.75 AS builder
ARG TARGETPLATFORM

# ... build steps ...
```

Build:
```bash
docker buildx build --platform linux/amd64,linux/arm64 \
    -t my-agent:latest --push .
```

## Best Practices

1. **Multi-stage builds** - Smaller final image
2. **Non-root user** - Security
3. **Health checks** - Container orchestration
4. **Dependency caching** - Faster builds
5. **Slim base images** - Reduced attack surface

