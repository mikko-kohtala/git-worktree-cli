//! Git Worktree CLI - A tool for managing git worktrees efficiently
//!
//! This library provides the core functionality for the git worktree CLI tool,
//! enabling easy creation, management, and removal of git worktrees.

pub mod bitbucket_api;
pub mod bitbucket_auth;
pub mod bitbucket_data_center_api;
pub mod bitbucket_data_center_auth;
pub mod cli;
pub mod commands;
pub mod completions;
pub mod config;
pub mod constants;
pub mod core;
pub mod error;
pub mod git;
pub mod github;
pub mod hooks;

// Re-export commonly used types
pub use cli::{Cli, Commands};
pub use error::{Error, Result};