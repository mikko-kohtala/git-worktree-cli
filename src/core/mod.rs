//! Core business logic for git worktree management
//!
//! This module contains the core functionality that is independent of the CLI
//! interface and external API providers.

pub mod project;
pub mod utils;

// Re-export commonly used types
pub use project::Project;