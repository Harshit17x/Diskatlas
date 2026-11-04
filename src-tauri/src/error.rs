use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Filesystem I/O error on '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Drive not found: {0}")]
    DriveNotFound(String),

    #[error("Permission denied for path: {0}")]
    PermissionDenied(String),

    #[error("System diagnostic error: {0}")]
    System(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 3)?;

        let code = match self {
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Io { .. } => "IO_ERROR",
            AppError::DriveNotFound(_) => "DRIVE_NOT_FOUND",
            AppError::PermissionDenied(_) => "PERMISSION_DENIED",
            AppError::System(_) => "SYSTEM_ERROR",
            AppError::Migration(_) => "MIGRATION_ERROR",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::ValidationFailed(_) => "VALIDATION_FAILED",
            AppError::SecurityViolation(_) => "SECURITY_VIOLATION",
        };

        state.serialize_field("code", code)?;
        state.serialize_field("message", &self.to_string())?;

        let details = match self {
            AppError::Io { path, .. } => Some(format!("Path: {}", path)),
            _ => None,
        };
        state.serialize_field("details", &details)?;

        state.end()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
