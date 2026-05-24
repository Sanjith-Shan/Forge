//! Real OpenAI-backed provider (compiled only with the `ai` feature).
//!
//! The API key and model are resolved by the caller (see [`crate::settings`])
//! and passed in explicitly. The key is never logged or persisted here.

use super::provider::LlmProvider;
use crate::error::ForgeError;

/// Calls the OpenAI Chat Completions API.
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// Build a provider from an explicit key and model.
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
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
