/// gRPC Compatible Status Codes
///
/// These constants are derived from the StatusCode enum defined in forpc.proto.
use super::protocol::StatusCode as PbStatusCode;

pub struct StatusCode;

impl StatusCode {
    /// Success
    pub const OK: i32 = PbStatusCode::Ok as i32;
    
    /// The operation was cancelled (typically by the caller).
    pub const CANCELLED: i32 = PbStatusCode::Cancelled as i32;
    
    /// Unknown error.
    pub const UNKNOWN: i32 = PbStatusCode::Unknown as i32;
    
    /// Client specified an invalid argument.
    pub const INVALID_ARGUMENT: i32 = PbStatusCode::InvalidArgument as i32;
    
    /// Deadline expired before operation could complete.
    pub const DEADLINE_EXCEEDED: i32 = PbStatusCode::DeadlineExceeded as i32;
    
    /// Some requested entity (e.g., file or directory) was not found.
    pub const NOT_FOUND: i32 = PbStatusCode::NotFound as i32;
    
    /// Some entity that we attempted to create (e.g., file or directory) already exists.
    pub const ALREADY_EXISTS: i32 = PbStatusCode::AlreadyExists as i32;
    
    /// The caller does not have permission to execute the specified operation.
    pub const PERMISSION_DENIED: i32 = PbStatusCode::PermissionDenied as i32;
    
    /// Some resource has been exhausted, perhaps a per-user quota, or perhaps the entire file system is out of space.
    pub const RESOURCE_EXHAUSTED: i32 = PbStatusCode::ResourceExhausted as i32;
    
    /// Operation was rejected because the system is not in a state required for the operation's execution.
    pub const FAILED_PRECONDITION: i32 = PbStatusCode::FailedPrecondition as i32;
    
    /// The operation was aborted, typically due to a concurrency issue like sequencer check failures, transaction aborts, etc.
    pub const ABORTED: i32 = PbStatusCode::Aborted as i32;
    
    /// Operation was attempted past the valid range.
    pub const OUT_OF_RANGE: i32 = PbStatusCode::OutOfRange as i32;
    
    /// Operation is not implemented or not supported/enabled in this service.
    pub const UNIMPLEMENTED: i32 = PbStatusCode::Unimplemented as i32;
    
    /// Internal errors. Means some invariants expected by underlying system has been broken.
    pub const INTERNAL: i32 = PbStatusCode::Internal as i32;
    
    /// The service is currently unavailable.
    pub const UNAVAILABLE: i32 = PbStatusCode::Unavailable as i32;
    
    /// Unrecoverable data loss or corruption.
    pub const DATA_LOSS: i32 = PbStatusCode::DataLoss as i32;
    
    /// The request does not have valid authentication credentials for the operation.
    pub const UNAUTHENTICATED: i32 = PbStatusCode::Unauthenticated as i32;
}

impl StatusCode {
    pub fn description(code: i32) -> &'static str {
        match PbStatusCode::try_from(code) {
            Ok(sc) => sc.as_str_name(),
            Err(_) => "UNKNOWN_STATUS_CODE",
        }
    }
}
