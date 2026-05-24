//! Backends: consume the validated [`Board`] IR and emit target artifacts.
//!
//! Every backend implements [`Backend`] and returns a list of in-memory
//! [`GeneratedFile`]s. A single async writer fans them out to disk, so the IR,
//! the analysis, and the file-writing machinery are shared across every target
//! — adding a new target (ESP-IDF, a linker script, …) is just another
//! `Backend` impl.

pub mod c;
pub mod zephyr;

use std::path::{Path, PathBuf};

use crate::error::ForgeError;
use crate::model::Board;

/// The dependency-resolved initialization order handed to a backend.
#[derive(Debug, Clone)]
pub struct InitPlan {
    /// Nodes grouped into layers by dependency depth (layer 0 has no deps).
    pub layers: Vec<Vec<String>>,
}

/// A file a backend wants written, with a path relative to the output dir.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// Path relative to the output directory (may contain subdirectories).
    pub path: PathBuf,
    /// File contents.
    pub contents: String,
}

impl GeneratedFile {
    /// Convenience constructor.
    pub fn new(path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            contents: contents.into(),
        }
    }
}

/// A code-generation target.
pub trait Backend {
    /// Stable identifier used on the command line (e.g. `"c"`, `"zephyr"`).
    fn id(&self) -> &'static str;

    /// A one-line human description of what this backend emits.
    fn description(&self) -> &'static str;

    /// Render the board into a set of output files. Pure: no I/O.
    fn render(&self, board: &Board, plan: &InitPlan) -> Vec<GeneratedFile>;
}

/// Every backend id Forge knows about.
pub const BACKEND_IDS: &[&str] = &["c", "zephyr"];

/// Resolve a backend by its id.
pub fn by_id(id: &str) -> Option<Box<dyn Backend>> {
    match id {
        "c" => Some(Box::new(c::CBackend)),
        "zephyr" => Some(Box::new(zephyr::ZephyrBackend)),
        _ => None,
    }
}

/// Write a backend's files into `output_dir`, creating directories as needed.
/// Files are written concurrently.
pub async fn write_files(files: Vec<GeneratedFile>, output_dir: &Path) -> Result<(), ForgeError> {
    tokio::fs::create_dir_all(output_dir)
        .await
        .map_err(|source| ForgeError::Write {
            path: output_dir.to_path_buf(),
            source,
        })?;

    let mut handles = Vec::with_capacity(files.len());
    for file in files {
        let full = output_dir.join(&file.path);
        handles.push(tokio::spawn(async move {
            if let Some(parent) = full.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|source| ForgeError::Write {
                        path: parent.to_path_buf(),
                        source,
                    })?;
            }
            tokio::fs::write(&full, file.contents)
                .await
                .map_err(|source| ForgeError::Write { path: full, source })
        }));
    }

    for handle in handles {
        handle.await??;
    }
    Ok(())
}
