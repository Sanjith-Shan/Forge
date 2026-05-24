//! Error types for the Forge pipeline.

use std::fmt;
use std::path::PathBuf;

/// A single, human-readable validation problem found in a `board.toml`.
///
/// Validation collects these into a `Vec` so every problem can be reported at
/// once, instead of forcing the user to fix issues one compile-cycle at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Actionable description of what is wrong and how to fix it.
    pub message: String,
}

impl ValidationError {
    /// Construct a validation error from anything string-like.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Top-level error returned by the Forge pipeline.
#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    /// The config file could not be read from disk.
    #[error("failed to read config file '{path}': {source}")]
    Io {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// The config file was not valid TOML.
    #[error("failed to parse TOML config: {0}")]
    Parse(#[from] toml::de::Error),

    /// The config parsed but failed validation. Carries every problem found.
    #[error("{}", render_validation(.0))]
    Validation(Vec<ValidationError>),

    /// A generated file could not be written to the output directory.
    #[error("failed to write output file '{path}': {source}")]
    Write {
        /// Path that could not be written.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// A spawned code-generation task panicked or was cancelled.
    #[error("code generation task failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    /// The AI frontend's language-model provider failed.
    #[error("AI provider error: {0}")]
    Ai(String),

    /// A command-line usage error (e.g. an unknown backend id).
    #[error("{0}")]
    Usage(String),
}

/// Render a list of validation errors as a numbered, human-readable block.
fn render_validation(errors: &[ValidationError]) -> String {
    use std::fmt::Write as _;

    let count = errors.len();
    let noun = if count == 1 { "error" } else { "errors" };
    let mut out = format!("configuration is invalid ({count} {noun}):");
    for (i, err) in errors.iter().enumerate() {
        // `write!` to a String is infallible; the result is ignored deliberately.
        let _ = write!(out, "\n  {}. {}", i + 1, err.message);
    }
    out
}
