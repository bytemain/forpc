/// gRPC Compatible Status Codes
pub struct StatusCode;

impl StatusCode {
    /// Success
    pub const OK: u32 = 0;
    
    /// The operation was cancelled (typically by the caller).
    pub const CANCELLED: u32 = 1;
    
    /// Unknown error.
    pub const UNKNOWN: u32 = 2;
    
    /// Client specified an invalid argument.
    pub const INVALID_ARGUMENT: u32 = 3;
    
    /// Deadline expired before operation could complete.
    pub const DEADLINE_EXCEEDED: u32 = 4;
    
    /// Some requested entity (e.g., file or directory) was not found.
    pub const NOT_FOUND: u32 = 5;
    
    /// Some entity that we attempted to create (e.g., file or directory) already exists.
    pub const ALREADY_EXISTS: u32 = 6;
    
    /// The caller does not have permission to execute the specified operation.
    pub const PERMISSION_DENIED: u32 = 7;
    
    /// Some resource has been exhausted, perhaps a per-user quota, or perhaps the entire file system is out of space.
    pub const RESOURCE_EXHAUSTED: u32 = 8;
    
    /// Operation was rejected because the system is not in a state required for the operation's execution.
    pub const FAILED_PRECONDITION: u32 = 9;
    
    /// The operation was aborted, typically due to a concurrency issue like sequencer check failures, transaction aborts, etc.
    pub const ABORTED: u32 = 10;
    
    /// Operation was attempted past the valid range.
    pub const OUT_OF_RANGE: u32 = 11;
    
    /// Operation is not implemented or not supported/enabled in this service.
    pub const UNIMPLEMENTED: u32 = 12;
    
    /// Internal errors. Means some invariants expected by underlying system has been broken.
    pub const INTERNAL: u32 = 13;
    
    /// The service is currently unavailable.
    pub const UNAVAILABLE: u32 = 14;
    
    /// Unrecoverable data loss or corruption.
    pub const DATA_LOSS: u32 = 15;
    
    /// The request does not have valid authentication credentials for the operation.
    pub const UNAUTHENTICATED: u32 = 16;
}

impl StatusCode {
    pub fn description(code: u32) -> &'static str {
        match code {
            0 => "OK",
            1 => "CANCELLED",
            2 => "UNKNOWN",
            3 => "INVALID_ARGUMENT",
            4 => "DEADLINE_EXCEEDED",
            5 => "NOT_FOUND",
            6 => "ALREADY_EXISTS",
            7 => "PERMISSION_DENIED",
            8 => "RESOURCE_EXHAUSTED",
            9 => "FAILED_PRECONDITION",
            10 => "ABORTED",
            11 => "OUT_OF_RANGE",
            12 => "UNIMPLEMENTED",
            13 => "INTERNAL",
            14 => "UNAVAILABLE",
            15 => "DATA_LOSS",
            16 => "UNAUTHENTICATED",
            _ => "UNKNOWN_STATUS_CODE",
        }
    }
}
