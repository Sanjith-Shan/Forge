//! The generic-C backend: the original five files of portable C scaffolding.
//!
//! This wraps the pure `render_*` functions in [`crate::codegen`] so the C
//! output is just one `Backend` among others.

use super::{Backend, GeneratedFile, InitPlan};
use crate::codegen;
use crate::model::Board;

/// Emits `init.{h,c}`, `handlers.{h,c}`, and `main.c`.
pub struct CBackend;

impl Backend for CBackend {
    fn id(&self) -> &'static str {
        "c"
    }

    fn description(&self) -> &'static str {
        "Portable C scaffolding with placeholder HAL macros"
    }

    fn render(&self, board: &Board, plan: &InitPlan) -> Vec<GeneratedFile> {
        vec![
            GeneratedFile::new("init.h", codegen::render_init_header(board)),
            GeneratedFile::new("init.c", codegen::render_init_source(board, &plan.layers)),
            GeneratedFile::new("handlers.h", codegen::render_handlers_header(board)),
            GeneratedFile::new("handlers.c", codegen::render_handlers_source(board)),
            GeneratedFile::new("main.c", codegen::render_main(board)),
        ]
    }
}
