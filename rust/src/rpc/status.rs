/// gRPC Compatible Status Codes
///
/// These constants are derived from the StatusCode enum defined in forpc.proto.
use super::protocol::StatusCode as PbStatusCode;

pub struct StatusCode;

impl StatusCode {
    /// Success
    pub const OK: u32 = PbStatusCode::Ok as u32;
    
    /// The operation was cancelled (typically by the caller).
    pub const CANCELLED: u32 = PbStatusCode::Cancelled as u32;
    
    /// Unknown error.
    pub const UNKNOWN: u32 = PbStatusCode::Unknown as u32;
    
    /// Client specified an invalid argument.
    pub const INVALID_ARGUMENT: u32 = PbStatusCode::InvalidArgument as u32;
    
    /// Deadline expired before operation could complete.
    pub const DEADLINE_EXCEEDED: u32 = PbStatusCode::DeadlineExceeded as u32;
    
    /// Some requested entity (e.g., file or directory) was not found.
    pub const NOT_FOUND: u32 = PbStatusCode::NotFound as u32;
    
    /// Some entity that we attempted to create (e.g., file or directory) already exists.
    pub const ALREADY_EXISTS: u32 = PbStatusCode::AlreadyExists as u32;
    
    /// The caller does not have permission to execute the specified operation.
    pub const PERMISSION_DENIED: u32 = PbStatusCode::PermissionDenied as u32;
    
    /// Some resource has been exhausted, perhaps a per-user quota, or perhaps the entire file system is out of space.
    pub const RESOURCE_EXHAUSTED: u32 = PbStatusCode::ResourceExhausted as u32;
    
    /// Operation was rejected because the system is not in a state required for the operation's execution.
    pub const FAILED_PRECONDITION: u32 = PbStatusCode::FailedPrecondition as u32;
    
    /// The operation was aborted, typically due to a concurrency issue like sequencer check failures, transaction aborts, etc.
    pub const ABORTED: u32 = PbStatusCode::Aborted as u32;
    
    /// Operation was attempted past the valid range.
    pub const OUT_OF_RANGE: u32 = PbStatusCode::OutOfRange as u32;
    
    /// Operation is not implemented or not supported/enabled in this service.
    pub const UNIMPLEMENTED: u32 = PbStatusCode::Unimplemented as u32;
    
    /// Internal errors. Means some invariants expected by underlying system has been broken.
    pub const INTERNAL: u32 = PbStatusCode::Internal as u32;
    
    /// The service is currently unavailable.
    pub const UNAVAILABLE: u32 = PbStatusCode::Unavailable as u32;
    
    /// Unrecoverable data loss or corruption.
    pub const DATA_LOSS: u32 = PbStatusCode::DataLoss as u32;
    
    /// The request does not have valid authentication credentials for the operation.
    pub const UNAUTHENTICATED: u32 = PbStatusCode::Unauthenticated as u32;
}

impl StatusCode {
    pub fn description(code: u32) -> &'static str {
        match PbStatusCode::try_from(code as i32) {
            Ok(sc) => sc.as_str_name(),
            Err(_) => "UNKNOWN_STATUS_CODE",
        }
    }
}
