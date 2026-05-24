//! The language-model provider abstraction.
//!
//! The AI frontend depends only on this trait, so it can run against a real
//! model or a deterministic mock. Tests use [`MockProvider`]; the CLI uses the
//! OpenAI-backed provider when built with the `ai` feature.

use crate::error::ForgeError;

/// Something that can turn a (system, user) prompt pair into a completion.
#[allow(async_fn_in_trait)] // providers are used through generics, not `dyn`.
pub trait LlmProvider {
    /// Produce a completion for the given prompts.
    async fn complete(&self, system: &str, user: &str) -> Result<String, ForgeError>;
}

/// A provider that returns a fixed, pre-baked response. For tests and offline
/// demonstration of the verification gate.
pub struct MockProvider {
    response: String,
}

impl MockProvider {
    /// Create a mock that always returns `response`.
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

impl LlmProvider for MockProvider {
    async fn complete(&self, _system: &str, _user: &str) -> Result<String, ForgeError> {
        Ok(self.response.clone())
    }
}
