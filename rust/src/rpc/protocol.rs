use prost::Message;
use std::collections::HashMap;
use bytes::{Bytes, BytesMut, Buf, BufMut};

use std::fmt;

pub mod frame_kind {
    pub const HEADERS: u8 = 0;
    pub const DATA: u8 = 1;
    pub const TRAILERS: u8 = 2;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketDecodeError {
    TooShort { len: usize },
}

impl fmt::Display for PacketDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { len } => write!(f, "packet too short: len={}", len),
        }
    }
}

impl std::error::Error for PacketDecodeError {}

#[derive(Debug, Clone)]
pub struct Packet {
    pub stream_id: u32,
    pub kind: u8,
    pub payload: Bytes,
}

impl Packet {
    pub fn headers(stream_id: u32, call: &Call) -> Self {
        Self {
            stream_id,
            kind: frame_kind::HEADERS,
            payload: Bytes::from(call.encode_to_vec()),
        }
    }
    
    pub fn data(stream_id: u32, payload: Bytes) -> Self {
        Self {
            stream_id,
            kind: frame_kind::DATA,
            payload,
        }
    }
    
    pub fn trailers(stream_id: u32, status: &Status) -> Self {
        Self {
            stream_id,
            kind: frame_kind::TRAILERS,
            payload: Bytes::from(status.encode_to_vec()),
        }
    }
    
    pub fn encode(self) -> Result<Bytes, PacketDecodeError> {
        // Manual framing: u32 (BE) + u8
        let mut buf = BytesMut::with_capacity(5 + self.payload.len());
        buf.put_u32(self.stream_id);
        buf.put_u8(self.kind);
        buf.extend_from_slice(&self.payload);
        Ok(buf.freeze())
    }
    
    pub fn decode(mut data: Bytes) -> Result<Self, PacketDecodeError> {
        if data.len() < 5 {
            return Err(PacketDecodeError::TooShort { len: data.len() });
        }
        let stream_id = data.get_u32();
        let kind = data.get_u8();
        // Remaining is payload
        // data.get_* advances the Bytes cursor (slice)
        let payload = data;
        
        Ok(Self {
            stream_id,
            kind,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_encode_decode_roundtrip() {
        let p = Packet::data(123, Bytes::from_static(b"hello"));
        let encoded = p.clone().encode().unwrap();
        let decoded = Packet::decode(encoded).unwrap();
        assert_eq!(decoded.stream_id, 123);
        assert_eq!(decoded.kind, frame_kind::DATA);
        assert_eq!(decoded.payload, Bytes::from_static(b"hello"));
    }

    #[test]
    fn packet_decode_too_short() {
        let err = Packet::decode(Bytes::from_static(b"\x01\x02\x03\x04")).unwrap_err();
        assert_eq!(err, PacketDecodeError::TooShort { len: 4 });
    }
}

include!("../gen/forpc.rs");

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
