#![deny(clippy::all)]

use std::ffi::CString;
use std::io::Write;
use std::sync::Arc;

use anng::protocols::reqrep0::{Rep0Raw, Req0Raw};
use anng::{AioError, Message, Socket};
use bytes::Bytes;
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use tokio::sync::Mutex;

#[napi]
pub fn plus_100(input: u32) -> u32 {
  input + 100
}

/// AsyncDealer - Client-side socket for request-reply pattern
/// Wraps the nng Req0Raw protocol for async operations
#[napi]
pub struct AsyncDealer {
  socket: Arc<Mutex<Socket<Req0Raw>>>,
  next_request_id: Arc<std::sync::atomic::AtomicU32>,
}

#[napi]
impl AsyncDealer {
  /// Dial to the specified URL and establish a connection
  #[napi(factory)]
  pub async fn dial(url: String) -> napi::Result<Self> {
    let c_url = CString::new(url).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    let socket = Req0Raw::dial(&c_url)
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Self {
      socket: Arc::new(Mutex::new(socket)),
      next_request_id: Arc::new(std::sync::atomic::AtomicU32::new(0x80000000)),
    })
  }

  /// Send a message and return the request ID
  #[napi]
  pub async fn send(&self, body: Buffer) -> napi::Result<u32> {
    let id = self
      .next_request_id
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let req_id = id | 0x80000000;

    let body_bytes = body.as_ref();
    let mut msg = Message::with_capacity(body_bytes.len());
    msg
      .write_all(body_bytes)
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    msg
      .write_header(&req_id.to_be_bytes())
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    let mut socket = self.socket.lock().await;
    socket
      .send(msg)
      .await
      .map_err(|(e, _): (AioError, _)| napi::Error::from_reason(format!("{:?}", e)))?;
    Ok(req_id)
  }

  /// Receive a message, returns DealerMessage
  #[napi]
  pub async fn recv(&self) -> napi::Result<DealerMessage> {
    let mut socket = self.socket.lock().await;
    let msg = socket
      .recv()
      .await
      .map_err(|e: AioError| napi::Error::from_reason(format!("{:?}", e)))?;
    Ok(DealerMessage { inner: msg })
  }
}

/// Message received from an AsyncDealer
#[napi]
pub struct DealerMessage {
  inner: Message,
}

#[napi]
impl DealerMessage {
  /// Get the message body as a Buffer
  #[napi]
  pub fn body(&self) -> Buffer {
    Buffer::from(self.inner.as_slice().to_vec())
  }

  /// Get the message header as a Buffer
  #[napi]
  pub fn header(&self) -> Buffer {
    Buffer::from(self.inner.header().to_vec())
  }
}

/// AsyncRouter - Server-side socket for request-reply pattern
/// Wraps the nng Rep0Raw protocol for async operations
#[napi]
pub struct AsyncRouter {
  socket: Arc<Mutex<Socket<Rep0Raw>>>,
}

#[napi]
impl AsyncRouter {
  /// Listen on the specified URL
  #[napi(factory)]
  pub async fn listen(url: String) -> napi::Result<Self> {
    let c_url = CString::new(url).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    let socket = Rep0Raw::listen(&c_url)
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Self {
      socket: Arc::new(Mutex::new(socket)),
    })
  }

  /// Receive a message from a client
  #[napi]
  pub async fn recv(&self) -> napi::Result<RouterMessage> {
    let mut socket = self.socket.lock().await;
    let msg = socket
      .recv()
      .await
      .map_err(|e: AioError| napi::Error::from_reason(format!("{:?}", e)))?;
    Ok(RouterMessage::from_msg(msg))
  }

  /// Send a response message back to the client
  #[napi]
  pub async fn send(&self, msg: &RouterMessage) -> napi::Result<()> {
    let out_msg = msg.to_msg()?;
    let mut socket = self.socket.lock().await;
    socket
      .send(out_msg)
      .await
      .map_err(|(e, _): (AioError, _)| napi::Error::from_reason(format!("{:?}", e)))?;
    Ok(())
  }
}

/// Message received from an AsyncRouter
/// Contains identity information for routing responses back to the correct client
#[napi]
pub struct RouterMessage {
  header: Bytes,
  body: Bytes,
}

impl RouterMessage {
  fn from_msg(msg: Message) -> Self {
    let header = Bytes::copy_from_slice(msg.header());
    let body = Bytes::copy_from_slice(msg.as_slice());
    Self { header, body }
  }

  fn to_msg(&self) -> napi::Result<Message> {
    let mut msg = Message::with_capacity(self.body.len());
    msg
      .write_all(&self.body)
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    msg
      .write_header(&self.header)
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(msg)
  }
}

impl Default for RouterMessage {
  fn default() -> Self {
    Self::new()
  }
}

#[napi]
impl RouterMessage {
  /// Get the client identity (first 4 bytes of header for routing)
  #[napi]
  pub fn identity(&self) -> Option<Buffer> {
    self.header.get(..4).map(|b| Buffer::from(b.to_vec()))
  }

  /// Get the full header as a Buffer
  #[napi]
  pub fn header(&self) -> Buffer {
    Buffer::from(self.header.to_vec())
  }

  /// Get the message body as a Buffer
  #[napi]
  pub fn body(&self) -> Buffer {
    Buffer::from(self.body.to_vec())
  }

  /// Set the message body (for creating a response)
  #[napi]
  pub fn set_body(&mut self, body: Buffer) {
    self.body = Bytes::from(body.to_vec());
  }

  /// Create a response message with the same header (for routing) but new body
  #[napi]
  pub fn create_response(&self, body: Buffer) -> RouterMessage {
    RouterMessage {
      header: self.header.clone(),
      body: Bytes::from(body.to_vec()),
    }
  }

  /// Create an empty RouterMessage (for manual construction)
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      header: Bytes::new(),
      body: Bytes::new(),
    }
  }
}
