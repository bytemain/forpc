use std::fmt;
use super::protocol::{Status, StatusCode};
use crate::BoxError;

#[derive(Debug)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub source: Option<BoxError>,
}

impl RpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
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
            x if x == StatusCode::Unavailable as i32
                || x == StatusCode::ResourceExhausted as i32
                || x == StatusCode::Aborted as i32
        )
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let desc = StatusCode::try_from(self.code)
            .map_or("UNKNOWN_STATUS_CODE", |c| c.as_str_name());
        write!(
            f,
            "RpcError {{ code: {} ({}), message: \"{}\" }}",
            self.code,
            desc,
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
