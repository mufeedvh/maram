//! Error types for maram
//!
//! This module defines all custom error types used throughout the application.
//! We use thiserror to derive Error trait implementations with zero runtime overhead.

use thiserror::Error;

/// Result type alias for maram operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for maram
#[derive(Error, Debug)]
pub enum Error {
    /// I/O errors from filesystem operations
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Regex compilation errors
    #[error("Invalid regex pattern: {0}")]
    RegexError(#[from] regex::Error),
    
    /// Configuration file parsing errors
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    /// TOML parsing errors
    #[error("TOML parsing error: {0}")]
    TomlError(#[from] toml::de::Error),
    
    /// JSON serialization errors
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    /// Size parsing errors
    #[error("Invalid size format: {0}")]
    SizeParseError(String),
    
    /// Time parsing errors
    #[error("Invalid time format: {0}")]
    TimeParseError(String),
    
    /// Thread pool errors
    #[error("Thread pool error: {0}")]
    ThreadPoolError(String),
    
    /// Path errors
    #[error("Invalid path: {0}")]
    PathError(String),
    
    /// Permission errors
    #[error("Permission denied: {0}")]
    PermissionError(String),
    
    /// Git-related errors
    #[error("Git error: {0}")]
    GitError(String),
    
    /// General errors
    #[error("{0}")]
    General(String),
}

impl Error {
    /// Create a new general error with a custom message
    pub fn general<S: Into<String>>(msg: S) -> Self {
        Error::General(msg.into())
    }
    
    /// Create a new configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Error::ConfigError(msg.into())
    }
    
    /// Create a new size parse error
    pub fn size_parse<S: Into<String>>(msg: S) -> Self {
        Error::SizeParseError(msg.into())
    }
    
    /// Create a new time parse error
    pub fn time_parse<S: Into<String>>(msg: S) -> Self {
        Error::TimeParseError(msg.into())
    }
    
    /// Create a new path error
    pub fn path<S: Into<String>>(msg: S) -> Self {
        Error::PathError(msg.into())
    }
    
    /// Create a new permission error
    pub fn permission<S: Into<String>>(msg: S) -> Self {
        Error::PermissionError(msg.into())
    }
}