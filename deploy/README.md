# Deployment Stubs

This directory will contain Terraform modules and IaC assets for shipping deep agents to AWS. Upcoming modules:

- `modules/runtime-lambda/`: Lambda-based execution with EventBridge scheduling.
- `modules/runtime-ecs/`: ECS service definition for long-running agents.
- `modules/data-plane/`: DynamoDB tables, S3 buckets, and CloudWatch dashboards.

Each module should include usage examples, required variables, and security considerations.
