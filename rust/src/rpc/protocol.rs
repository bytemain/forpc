use prost::Message;
use std::collections::HashMap;
use bytes::Bytes;

include!("../gen/forpc.rs");

/// Frame kind constants (i32 to match the generated `FrameKind` proto enum).
///
/// These mirror the [`FrameKind`] enum values and are kept as plain
/// constants for ergonomic comparison against `Packet::kind` (which is
/// `i32` in the generated code).
pub mod frame_kind {
    pub const HEADERS: i32 = 0;
    pub const DATA: i32 = 1;
    pub const TRAILERS: i32 = 2;
    pub const RST_STREAM: i32 = 3;
}

impl Packet {
    /// Build a HEADERS frame carrying an encoded [`Call`].
    pub fn headers(stream_id: u32, call: &Call) -> Self {
        Self {
            stream_id,
            kind: frame_kind::HEADERS,
            payload: call.encode_to_vec(),
            error_code: 0,
        }
    }

    /// Build a DATA frame carrying user payload bytes.
    pub fn data(stream_id: u32, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            stream_id,
            kind: frame_kind::DATA,
            payload: payload.into(),
            error_code: 0,
        }
    }

    /// Build a TRAILERS frame carrying an encoded [`Status`].
    pub fn trailers(stream_id: u32, status: &Status) -> Self {
        Self {
            stream_id,
            kind: frame_kind::TRAILERS,
            payload: status.encode_to_vec(),
            error_code: 0,
        }
    }

    /// Build a RST_STREAM frame with the given error code.
    pub fn rst_stream(stream_id: u32, error_code: u32) -> Self {
        Self {
            stream_id,
            kind: frame_kind::RST_STREAM,
            payload: Vec::new(),
            error_code,
        }
    }

    /// Encode this packet to wire bytes using protobuf.
    pub fn encode_to_bytes(&self) -> Bytes {
        Bytes::from(self.encode_to_vec())
    }

    /// Decode a packet from wire bytes using protobuf.
    pub fn decode_from_bytes(buf: &[u8]) -> Result<Self, prost::DecodeError> {
        <Self as Message>::decode(buf)
    }

    /// Take the payload, leaving an empty `Vec` in its place.
    pub fn take_payload(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_encode_decode_roundtrip() {
        let p = Packet::data(123, Vec::from(&b"hello"[..]));
        let encoded = p.encode_to_bytes();
        let decoded = Packet::decode_from_bytes(&encoded).unwrap();
        assert_eq!(decoded.stream_id, 123);
        assert_eq!(decoded.kind, frame_kind::DATA);
        assert_eq!(decoded.payload, b"hello");
    }

    #[test]
    fn rst_stream_carries_error_code() {
        let p = Packet::rst_stream(7, StatusCode::Cancelled as u32);
        let encoded = p.encode_to_bytes();
        let decoded = Packet::decode_from_bytes(&encoded).unwrap();
        assert_eq!(decoded.stream_id, 7);
        assert_eq!(decoded.kind, frame_kind::RST_STREAM);
        assert_eq!(decoded.error_code, StatusCode::Cancelled as u32);
        assert!(decoded.payload.is_empty());
    }
}

impl Call {
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
    
    pub fn timeout_ms(&self) -> Option<u64> {
        self.metadata.get(":timeout").and_then(|v| v.parse().ok())
    }
}

impl Status {
    pub fn new(code: StatusCode, message: impl Into<String>) -> Self {
        Self { code: code as i32, message: message.into() }
    }
    
    pub fn ok() -> Self {
        Self::new(StatusCode::Ok, "OK")
    }
    
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Internal, message)
    }
    
    pub fn is_ok(&self) -> bool {
        self.code == StatusCode::Ok as i32
    }
}
