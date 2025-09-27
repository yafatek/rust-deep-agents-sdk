pub mod openai;

use std::sync::Arc;

use agents_core::llm::{LanguageModel, LlmRequest, LlmResponse};
#[derive(Clone)]
pub enum LlmProvider {
    OpenAi(Arc<dyn LanguageModel>),
}

impl LlmProvider {
    pub async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse> {
        match self {
            LlmProvider::OpenAi(model) => model.generate(request).await,
        }
    }
}
