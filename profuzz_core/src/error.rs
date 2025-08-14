#![allow(missing_docs)]
use thiserror::Error;
use tokio::time::error::Elapsed;

/// Custom Result type used in this crate
pub(crate) type ProFuzzResult<T> = Result<T, ProFuzzError>;

#[derive(Debug, Error)]
pub enum ProFuzzError {
    #[error("{err_msg}")]
    Custom { err_msg: String },
    #[error("Could not connect to {err_msg}")]
    ConnectionFailed { err_msg: String },
    #[error("Could not write to the transport")]
    TransporterWrite,
    #[error("Could not read from the transport")]
    TransporterRead,
    #[error("Command line error {command}")]
    CommandLineError { command: String },
    #[error("Output directory already exists, please enable `AUTO_RESUME` to resume the session.")]
    AutoResumeNotEnabled,
    #[error("Run into a timeout {elapsed}.")]
    Timeout { elapsed: Elapsed },
    #[error("{err}")]
    IoError { err: std::io::Error },
    #[error("{err}")]
    Serde { err: serde_json::Error },
}

impl From<std::io::Error> for ProFuzzError {
    fn from(err: std::io::Error) -> Self {
        ProFuzzError::IoError { err }
    }
}

impl From<serde_json::Error> for ProFuzzError {
    fn from(err: serde_json::Error) -> Self {
        ProFuzzError::Serde { err }
    }
}

impl From<Elapsed> for ProFuzzError {
    fn from(elapsed: Elapsed) -> Self {
        ProFuzzError::Timeout { elapsed }
    }
}
