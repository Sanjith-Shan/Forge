//! Real OpenAI-backed provider (compiled only with the `ai` feature).
//!
//! Reads `OPENAI_API_KEY` from the environment (and optionally `OPENAI_MODEL`).
//! The key is never logged or persisted.

use super::provider::LlmProvider;
use crate::error::ForgeError;

/// Default model if `OPENAI_MODEL` is not set.
const DEFAULT_MODEL: &str = "gpt-4o-mini";

/// Calls the OpenAI Chat Completions API.
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// Build a provider from environment variables.
    ///
    /// Requires `OPENAI_API_KEY`. Honors `OPENAI_MODEL` (defaults to
    /// `gpt-4o-mini`).
    pub fn from_env() -> Result<Self, ForgeError> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            ForgeError::Ai("OPENAI_API_KEY is not set in the environment".to_string())
        })?;
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        Ok(Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        })
    }
}

impl LlmProvider for OpenAiProvider {
    async fn complete(&self, system: &str, user: &str) -> Result<String, ForgeError> {
        let body = serde_json::json!({
            "model": self.model,
            "temperature": 0,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ForgeError::Ai(format!("request failed: {e}")))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| ForgeError::Ai(format!("reading response failed: {e}")))?;

        if !status.is_success() {
            return Err(ForgeError::Ai(format!("OpenAI returned {status}: {text}")));
        }

        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| ForgeError::Ai(format!("invalid JSON response: {e}")))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| ForgeError::Ai("response had no message content".to_string()))
    }
}
