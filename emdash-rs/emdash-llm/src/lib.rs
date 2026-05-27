use async_trait::async_trait;
use emdash_core::{ApiError, ChatMessage, LlmOptions, LlmProvider};
use serde::{Deserialize, Serialize};

// ── OpenAI wire types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest<'a> {
    model:       &'a str,
    messages:    &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens:  Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop:        Option<&'a [String]>,
    stream:      bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

#[derive(Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

// ── Provider ──────────────────────────────────────────────────────────────────

/// Universal OpenAI-compatible LLM provider.
///
/// Works with any endpoint that speaks the OpenAI REST API — OpenAI,
/// Ollama, Groq, Together, LM Studio, local llama.cpp, etc.
///
/// Configure via environment variables:
/// - `LLM_BASE_URL`  (default: `https://api.openai.com`)
/// - `LLM_API_KEY`   (optional)
/// - `LLM_MODEL`     (default: `gpt-4o-mini`)
pub struct OpenAiCompatProvider {
    base_url: String,
    api_key:  Option<String>,
    model:    String,
    client:   reqwest::Client,
}

impl OpenAiCompatProvider {
    pub fn new(base_url: impl Into<String>, api_key: Option<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Convenience constructor that reads env vars.
    pub fn from_env() -> Self {
        Self::new(
            std::env::var("LLM_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".into()),
            std::env::var("LLM_API_KEY").ok(),
            std::env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into()),
        )
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(key) => req.bearer_auth(key),
            None      => req,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatProvider {
    async fn generate_text(&self, prompt: &str, opts: LlmOptions) -> Result<String, ApiError> {
        self.chat_completion(
            vec![ChatMessage { role: "user".into(), content: prompt.into() }],
            opts,
        )
        .await
    }

    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        opts: LlmOptions,
    ) -> Result<String, ApiError> {
        let body = ChatRequest {
            model:       &self.model,
            messages:    &messages,
            temperature: opts.temperature,
            max_tokens:  opts.max_tokens,
            stop:        opts.stop.as_deref(),
            stream:      false,
        };

        let resp = self
            .auth(self.client.post(format!("{}/v1/chat/completions", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("llm request: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ApiError::Internal(format!("llm {status}: {text}")));
        }

        let parsed: ChatResponse = resp
            .json()
            .await
            .map_err(|e| ApiError::Internal(format!("llm parse: {e}")))?;

        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| ApiError::Internal("llm returned no choices".into()))
    }

    async fn embed(&self, input: &str) -> Result<Vec<f32>, ApiError> {
        let body = EmbedRequest { model: &self.model, input };

        let resp = self
            .auth(self.client.post(format!("{}/v1/embeddings", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("embed request: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ApiError::Internal(format!("embed {status}: {text}")));
        }

        let parsed: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| ApiError::Internal(format!("embed parse: {e}")))?;

        parsed
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| ApiError::Internal("embed returned no data".into()))
    }
}
