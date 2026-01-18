use std::fmt;
use super::status::StatusCode;
use super::protocol::Status;
use crate::BoxError;

#[derive(Debug)]
pub struct RpcError {
    pub code: u32,
    pub message: String,
    pub source: Option<BoxError>,
}

impl RpcError {
    pub fn new(code: u32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }
    
    pub fn from_status(status: Status) -> Self {
        Self {
            code: status.code,
            message: status.message,
            source: None,
        }
    }
    
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }
    
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.code,
            StatusCode::UNAVAILABLE | StatusCode::RESOURCE_EXHAUSTED | StatusCode::ABORTED
        )
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RpcError {{ code: {} ({}), message: \"{}\" }}",
            self.code,
            StatusCode::description(self.code),
            self.message
        )
    }
}

impl std::error::Error for RpcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

pub type RpcResult<T> = Result<T, RpcError>;
