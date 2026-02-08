use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoofError {
    #[error("Platform error: {0}")]
    Platform(String),
    #[error("Process not found: PID {0}")]
    ProcessNotFound(u32),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, LoofError>;
