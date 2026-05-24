//! The AI frontend: natural language / datasheet text → a *verified* config.
//!
//! The model only ever proposes a candidate `board.toml`. That candidate is run
//! through the exact same deterministic gate as a hand-written file — TOML
//! parsing, [`crate::config::validate`], [`crate::graph::build`], and the
//! [`crate::lint`] review — so the language model can never smuggle an invalid
//! design past Forge. The model proposes; the deterministic core disposes.

pub mod prompt;
pub mod provider;

#[cfg(feature = "ai")]
pub mod openai;

pub use provider::{LlmProvider, MockProvider};

use crate::config;
use crate::error::{ForgeError, ValidationError};
use crate::graph;
use crate::lint::{self, Diagnostic};
use crate::model::Board;

/// What the user asked the AI to build.
#[derive(Debug, Clone)]
pub struct AiRequest {
    /// Natural-language description of the board.
    pub intent: String,
    /// Optional datasheet text to ground the model.
    pub datasheet: Option<String>,
}

/// The result of synthesizing and verifying a candidate config.
#[derive(Debug, Clone)]
pub struct AiOutcome {
    /// The TOML the model proposed (post fence-stripping).
    pub candidate_toml: String,
    /// The validated board, if it passed every gate.
    pub board: Option<Board>,
    /// Hard errors (parse / validation / graph) that rejected the candidate.
    pub errors: Vec<ValidationError>,
    /// Advisory lint diagnostics (only when the candidate is valid).
    pub diagnostics: Vec<Diagnostic>,
}

impl AiOutcome {
    /// True if the candidate passed every hard gate.
    pub fn is_valid(&self) -> bool {
        self.board.is_some()
    }
}

/// Ask `provider` for a config and run it through the verification gate.
pub async fn synthesize<P: LlmProvider>(
    provider: &P,
    request: &AiRequest,
) -> Result<AiOutcome, ForgeError> {
    let reply = provider
        .complete(&prompt::system_prompt(), &prompt::user_prompt(request))
        .await?;
    let candidate = extract_toml(&reply);
    Ok(verify(candidate))
}

/// Run a candidate TOML string through parse → validate → graph → lint.
pub fn verify(candidate: String) -> AiOutcome {
    let raw = match config::parse_str(&candidate) {
        Ok(raw) => raw,
        Err(e) => {
            return AiOutcome {
                candidate_toml: candidate,
                board: None,
                errors: vec![ValidationError::new(format!(
                    "the model did not produce valid TOML: {e}"
                ))],
                diagnostics: Vec::new(),
            };
        }
    };

    let validated = match config::validate(raw) {
        Ok(v) => v,
        Err(errors) => {
            return AiOutcome {
                candidate_toml: candidate,
                board: None,
                errors,
                diagnostics: Vec::new(),
            };
        }
    };

    if let Err(errors) = graph::build(&validated.board) {
        return AiOutcome {
            candidate_toml: candidate,
            board: None,
            errors,
            diagnostics: Vec::new(),
        };
    }

    let diagnostics = lint::review(&validated.board);
    AiOutcome {
        candidate_toml: candidate,
        board: Some(validated.board),
        errors: Vec::new(),
        diagnostics,
    }
}

/// Pull the TOML out of a model reply, tolerating Markdown code fences.
fn extract_toml(reply: &str) -> String {
    let trimmed = reply.trim();
    if let Some(start) = trimmed.find("```") {
        // Skip the opening fence and an optional language tag on that line.
        let after_fence = &trimmed[start + 3..];
        let body_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
        let body = &after_fence[body_start..];
        if let Some(end) = body.find("```") {
            return body[..end].trim().to_string();
        }
        return body.trim().to_string();
    }
    trimmed.to_string()
}
