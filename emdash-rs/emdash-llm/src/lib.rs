use async_trait::async_trait;
use emdash_core::LlmProvider;

pub struct OpenAiCompatProvider {
    // Add base_url and api_key here in the future to avoid vendor lock-in
}

impl OpenAiCompatProvider {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatProvider {
    async fn generate_text(&self, _prompt: &str) -> Result<String, String> {
        Ok(String::new())
    }
}
