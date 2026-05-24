//! User settings and credential resolution.
//!
//! Forge reads optional defaults from `$XDG_CONFIG_HOME/forge/config.toml`
//! (falling back to `~/.config/forge/config.toml`). The file can hold an OpenAI
//! key and model plus default build options, but it is never required — every
//! value has a sensible fallback, and the API key can also come from the
//! environment or a flag.
//!
//! Credential precedence (highest first): explicit flag → environment variable
//! → config file. This keeps secrets out of source control while still allowing
//! a persistent local setup.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::ForgeError;

/// The default OpenAI model when none is configured.
pub const DEFAULT_MODEL: &str = "gpt-4o-mini";
/// The default backend when none is configured.
pub const DEFAULT_BACKEND: &str = "c";
/// The default output directory when none is configured.
pub const DEFAULT_OUTPUT: &str = "./output";

/// Persistent user settings, all optional.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// OpenAI API key (prefer the environment or a `.env` for this).
    pub openai_api_key: Option<String>,
    /// OpenAI model id.
    pub openai_model: Option<String>,
    /// Default backend for `build` (`c`, `zephyr`, `all`).
    pub default_backend: Option<String>,
    /// Default output directory for `build`.
    pub default_output: Option<String>,
}

/// The directory holding Forge's config file.
pub fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("forge");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("forge")
}

/// The full path to Forge's `config.toml`.
pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Load settings, returning defaults if the file is missing or unreadable.
pub fn load() -> Settings {
    match std::fs::read_to_string(config_path()) {
        Ok(text) => toml::from_str(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Write settings to the config file (creating the directory), restricting it to
/// the current user on Unix.
pub fn save(settings: &Settings) -> Result<(), ForgeError> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).map_err(|source| ForgeError::Io {
        path: dir.clone(),
        source,
    })?;
    let path = config_path();
    let body = toml::to_string_pretty(settings)
        .map_err(|e| ForgeError::Usage(format!("could not serialize settings: {e}")))?;
    std::fs::write(&path, body).map_err(|source| ForgeError::Io {
        path: path.clone(),
        source,
    })?;
    restrict_permissions(&path);
    Ok(())
}

/// Tighten file permissions to owner-only on Unix; a no-op elsewhere.
fn restrict_permissions(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// Resolve the OpenAI API key by precedence: flag → env → config file.
///
/// `env` is passed in (rather than read here) so the logic is pure and testable.
pub fn resolve_api_key(
    cli: Option<&str>,
    env: Option<&str>,
    settings: &Settings,
) -> Option<String> {
    non_empty(cli)
        .or_else(|| non_empty(env))
        .map(str::to_string)
        .or_else(|| settings.openai_api_key.clone().filter(|s| !s.is_empty()))
}

/// Resolve the OpenAI model by precedence: flag → env → config file → default.
pub fn resolve_model(cli: Option<&str>, env: Option<&str>, settings: &Settings) -> String {
    non_empty(cli)
        .or_else(|| non_empty(env))
        .map(str::to_string)
        .or_else(|| settings.openai_model.clone().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

/// Resolve the default backend: flag → config file → built-in default.
pub fn resolve_backend(cli: Option<&str>, settings: &Settings) -> String {
    non_empty(cli)
        .map(str::to_string)
        .or_else(|| settings.default_backend.clone().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| DEFAULT_BACKEND.to_string())
}

/// Resolve the default output dir: flag → config file → built-in default.
pub fn resolve_output(cli: Option<&str>, settings: &Settings) -> String {
    non_empty(cli)
        .map(str::to_string)
        .or_else(|| settings.default_output.clone().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| DEFAULT_OUTPUT.to_string())
}

/// Mask a secret for display, revealing only a short prefix and suffix.
pub fn redact(secret: &str) -> String {
    let n = secret.chars().count();
    if n <= 10 {
        "********".to_string()
    } else {
        let prefix: String = secret.chars().take(6).collect();
        let suffix: String = secret.chars().skip(n - 4).collect();
        format!("{prefix}…{suffix} ({n} chars)")
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|s| !s.is_empty())
}
