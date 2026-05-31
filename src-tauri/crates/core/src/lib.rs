//! blitzscript core: everything that does NOT depend on the GUI shell.
//!
//! This crate is deliberately Tauri-free so it compiles and tests in isolation
//! (no system webkit/gtk needed) and so the indexing logic can be reused or
//! swapped without touching the app shell. New discovery sources plug in here.

pub mod db;
pub mod discovery;
pub mod safety;
