//! Frontends: ways to produce a board configuration.
//!
//! The TOML frontend lives in [`crate::config`] (parse + validate). The [`ai`]
//! frontend turns natural language or datasheet text into a candidate config
//! and runs it through the same verification gate.

pub mod ai;
