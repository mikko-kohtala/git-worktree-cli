//! Error types for the git worktree CLI
//!
//! This module defines the centralized error handling for the application,
//! providing context-rich error messages for better debugging and user experience.

use thiserror::Error;

/// The main error type for the git worktree CLI
#[derive(Error, Debug)]
pub enum Error {
    /// IO errors from file system operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Git command execution errors
    #[error("Git command failed: {0}")]
    Git(String),

    /// Configuration parsing or validation errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// API provider errors (GitHub, Bitbucket, etc.)
    #[error("API provider error: {0}")]
    Provider(String),

    /// Project root or git directory not found
    #[error("Project root not found")]
    ProjectRootNotFound,

    /// Git directory not found
    #[error("Git directory not found in project")]
    GitDirectoryNotFound,

    /// Branch operation errors
    #[error("Branch operation failed: {0}")]
    Branch(String),

    /// Hook execution errors
    #[error("Hook execution failed: {0}")]
    Hook(String),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Generic errors with context
    #[error("{0}")]
    Other(String),
}

/// Type alias for Results with our Error type
pub type Result<T> = std::result::Result<T, Error>;

// Convenience functions for creating errors
impl Error {
    /// Create a generic error with a message
    pub fn msg<S: Into<String>>(msg: S) -> Self {
        Error::Other(msg.into())
    }

    /// Create a git error
    pub fn git<S: Into<String>>(msg: S) -> Self {
        Error::Git(msg.into())
    }

    /// Create a configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Error::Config(msg.into())
    }

    /// Create a branch error
    pub fn branch<S: Into<String>>(msg: S) -> Self {
        Error::Branch(msg.into())
    }
}

// Helper implementations for common conversions
impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Self {
        Error::Config(err.to_string())
    }
}

impl From<keyring::Error> for Error {
    fn from(err: keyring::Error) -> Self {
        Error::Auth(err.to_string())
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}