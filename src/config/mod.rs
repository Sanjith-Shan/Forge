//! Configuration loading: parsing `board.toml` and validating it into a
//! [`crate::model::Board`].

pub mod parse;
pub mod validate;

use std::path::Path;

pub use parse::{parse_str, RawConfig};
pub use validate::{validate, Validated};

use crate::ForgeError;

/// Read and parse a `board.toml` from disk.
///
/// This performs TOML deserialization only; semantic validation happens in
/// [`validate`].
pub fn parse(path: &Path) -> Result<RawConfig, ForgeError> {
    let contents = std::fs::read_to_string(path).map_err(|source| ForgeError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_str(&contents)
}
