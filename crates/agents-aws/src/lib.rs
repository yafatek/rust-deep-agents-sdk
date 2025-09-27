//! AWS integration helpers: wiring for Secrets Manager, DynamoDB, and CloudWatch.
//! Concrete implementations will live behind feature flags, so the core remains
//! lightweight when running outside AWS.

/// Placeholder trait for loading configuration secrets.
pub trait SecretsProvider {
    fn fetch(&self, key: &str) -> anyhow::Result<String>;
}

/// Stub Secrets Manager provider; real implementation will sit behind the `aws-sdk` feature.
pub struct UnimplementedSecretsProvider;

impl SecretsProvider for UnimplementedSecretsProvider {
    fn fetch(&self, key: &str) -> anyhow::Result<String> {
        Err(anyhow::anyhow!("Secrets provider not implemented: {key}"))
    }
}
