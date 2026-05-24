//! Forge — generate embedded C boilerplate from a TOML hardware description.
//!
//! The library is organised as a pipeline:
//!
//! 1. [`config::parse`] reads a `board.toml` into raw, lenient structs.
//! 2. [`config::validate`] checks those structs and lowers them into the typed
//!    [`model::Board`], collecting *all* problems rather than failing fast.
//! 3. [`codegen`] renders the validated board into C source files.
//!
//! The CLI in `main.rs` is a thin wrapper that wires these stages together and
//! runs the code generators concurrently with Tokio.

pub mod analysis;
pub mod backend;
pub mod codegen;
pub mod config;
pub mod error;
pub mod frontend;
pub mod graph;
pub mod lint;
pub mod model;

pub use error::{ForgeError, ValidationError};
pub use model::Board;
