use prost::Message;
use std::collections::HashMap;
use bytes::{Bytes, BytesMut, Buf, BufMut};

use std::fmt;

pub mod frame_kind {
    pub const HEADERS: u8 = 0;
    pub const DATA: u8 = 1;
    pub const TRAILERS: u8 = 2;
    pub const RST_STREAM: u8 = 3;
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

    pub fn rst_stream(stream_id: u32, error_code: u32) -> Self {
        let mut buf = BytesMut::with_capacity(4);
        buf.put_u32(error_code);
        Self {
            stream_id,
            kind: frame_kind::RST_STREAM,
            payload: buf.freeze(),
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

    /// Encode multiple packets into a single batch buffer.
    ///
    /// If there is only one packet, it is encoded normally (no batch overhead).
    /// Otherwise, the batch format uses `stream_id=0` as a sentinel:
    ///
    /// ```text
    /// [0x00000000: u32 BE]  -- batch sentinel
    /// [count: u32 BE]       -- number of sub-packets
    /// For each sub-packet:
    ///   [len: u32 BE]       -- length of encoded packet bytes
    ///   [packet bytes...]   -- standard packet format
    /// ```
    pub fn encode_batch(packets: Vec<Packet>) -> Result<Bytes, PacketDecodeError> {
        if packets.len() == 1 {
            return packets.into_iter().next().unwrap().encode();
        }
        let encoded: Vec<Bytes> = packets
            .into_iter()
            .map(|p| p.encode())
            .collect::<Result<Vec<_>, _>>()?;
        let total: usize = 8 + encoded.iter().map(|e| 4 + e.len()).sum::<usize>();
        let mut buf = BytesMut::with_capacity(total);
        buf.put_u32(0); // batch sentinel (stream_id = 0)
        buf.put_u32(encoded.len() as u32);
        for enc in &encoded {
            buf.put_u32(enc.len() as u32);
            buf.extend_from_slice(enc);
        }
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

    /// Decode a buffer that may contain a single packet or a batch of packets.
    ///
    /// If the first 4 bytes are `0x00000000` (stream_id = 0), the buffer is
    /// treated as a batch envelope and all sub-packets are returned.
    /// If batch decoding fails (e.g. a regular packet with stream_id=0),
    /// it falls back to single-packet decoding.
    pub fn decode_packets(data: Bytes) -> Result<Vec<Self>, PacketDecodeError> {
        if data.len() < 5 {
            return Err(PacketDecodeError::TooShort { len: data.len() });
        }
        // Peek at first 4 bytes to check for batch sentinel
        let stream_id = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        if stream_id == 0 {
            // Try batch decode first; fall back to single packet on failure
            if let Ok(packets) = Self::try_decode_batch(data.clone()) {
                return Ok(packets);
            }
        }
        Ok(vec![Self::decode(data)?])
    }

    /// Attempt to decode a batch envelope starting with sentinel stream_id=0.
    fn try_decode_batch(mut data: Bytes) -> Result<Vec<Self>, PacketDecodeError> {
        if data.len() < 8 {
            return Err(PacketDecodeError::TooShort { len: data.len() });
        }
        data.advance(4); // skip sentinel
        let count = data.get_u32() as usize;
        // Sanity check: count must be at least 2 for a batch (single packets
        // are not wrapped in batch envelopes by encode_batch)
        if count < 2 {
            return Err(PacketDecodeError::TooShort { len: 0 });
        }
        let mut packets = Vec::with_capacity(count);
        for _ in 0..count {
            if data.len() < 4 {
                return Err(PacketDecodeError::TooShort { len: data.len() });
            }
            let len = data.get_u32() as usize;
            if data.len() < len || len < 5 {
                return Err(PacketDecodeError::TooShort { len: data.len() });
            }
            let pkt_data = data.split_to(len);
            packets.push(Self::decode(pkt_data)?);
        }
        Ok(packets)
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

    #[test]
    fn encode_batch_single_packet_no_envelope() {
        let p = Packet::data(42, Bytes::from_static(b"single"));
        let single_encoded = p.clone().encode().unwrap();
        let batch_encoded = Packet::encode_batch(vec![p]).unwrap();
        assert_eq!(single_encoded, batch_encoded);
    }

    #[test]
    fn encode_decode_batch_roundtrip() {
        let p1 = Packet::headers(1, &Call::new("method1"));
        let p2 = Packet::data(1, Bytes::from_static(b"payload"));
        let p3 = Packet::trailers(1, &Status::ok());
        let encoded = Packet::encode_batch(vec![p1, p2, p3]).unwrap();
        // Batch should start with sentinel stream_id=0
        assert_eq!(&encoded[..4], &[0, 0, 0, 0]);
        let decoded = Packet::decode_packets(encoded).unwrap();
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0].kind, frame_kind::HEADERS);
        assert_eq!(decoded[0].stream_id, 1);
        assert_eq!(decoded[1].kind, frame_kind::DATA);
        assert_eq!(decoded[1].payload, Bytes::from_static(b"payload"));
        assert_eq!(decoded[2].kind, frame_kind::TRAILERS);
    }

    #[test]
    fn decode_packets_single_packet() {
        let p = Packet::data(99, Bytes::from_static(b"solo"));
        let encoded = p.encode().unwrap();
        let decoded = Packet::decode_packets(encoded).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].stream_id, 99);
        assert_eq!(decoded[0].payload, Bytes::from_static(b"solo"));
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
