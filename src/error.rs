//! Structured error types for the application.
//!
//! [`StorageError`] covers data-load/corruption/save failures.
//! [`CliError`] covers argument parsing and resolution errors,
//! and can convert from [`StorageError`] via `#[from]`.

use std::io;
use thiserror::Error;

/// Storage-layer errors (load/corruption/save).
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Failed to create config directory: {0}")]
    CreateDir(String),
    #[error("Corrupted data file `{path}`, backed up to `{backup}`: {detail}")]
    Corrupted {
        path: String,
        backup: String,
        detail: String,
    },
    #[error("Failed to read data file: {0}")]
    ReadFailed(String),
}

/// CLI parsing/resolution errors.
#[derive(Debug, Error)]
pub enum CliError {
    #[error("Invalid UUID: {0}")]
    InvalidUuid(String),
    #[error("No command set with UUID {0}")]
    SetNotFound(String),
    #[error("No command set found for group '{group}' set '{set}'")]
    SetByGroupNotFound { group: String, set: String },
    #[error("Ambiguous: found {count} matches:\n{detail}")]
    Ambiguous { count: usize, detail: String },
    #[error("Invalid --var format '{0}' (expected key=value)")]
    InvalidVar(String),
    #[error("Missing argument: specify --id or --group --set")]
    MissingArgs,
    #[error(transparent)]
    Storage(#[from] StorageError),
}
