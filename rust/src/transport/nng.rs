use anng::{Message, protocols::reqrep0::Rep0Raw, protocols::reqrep0::Req0Raw};
use std::ffi::CString;
use std::io::Write; 
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use bytes::Bytes;
use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex};
use crate::BoxError;

#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, data: Bytes) -> Result<(), BoxError>;
    async fn recv(&self) -> Result<Bytes, BoxError>;
}

pub struct AsyncDealer {
    socket: anng::Socket<Req0Raw>,
    next_request_id: AtomicU32,
}

impl AsyncDealer {
    pub async fn dial(url: &str) -> Result<Self, BoxError> {
        let c_url = CString::new(url)?;
        let socket = Req0Raw::dial(&c_url).await?;
        Ok(Self {
            socket,
            next_request_id: AtomicU32::new(0x80000000),
        })
    }

    pub async fn dial_with_retry(url: &str) -> Result<Self, BoxError> {
        Self::dial_with_retry_params(url, 10, Duration::from_millis(100)).await
    }

    pub async fn dial_with_retry_params(
        url: &str,
        max_retries: u32,
        retry_delay: Duration,
    ) -> Result<Self, BoxError> {
        let mut last_err: Option<BoxError> = None;
        let attempts = max_retries.saturating_add(1);
        for attempt in 0..attempts {
            if attempt > 0 {
                tokio::time::sleep(retry_delay).await;
            }
            match Self::dial(url).await {
                Ok(d) => return Ok(d),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| "Dial failed".into()))
    }

    pub async fn send(&mut self, body: &[u8]) -> Result<u32, BoxError> {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let req_id = id | 0x80000000;

        let mut msg = Message::with_capacity(body.len());
        msg.write_all(body)?;
        msg.write_header(&req_id.to_be_bytes())?;

        self.socket.send(msg).await.map_err(|(e, _)| e)?;
        Ok(req_id)
    }

    pub async fn recv(&mut self) -> Result<DealerMessage, BoxError> {
        let msg = self.socket.recv().await?;
        Ok(DealerMessage::from_msg(msg))
    }
}

pub struct AsyncRouter {
    socket: anng::Socket<Rep0Raw>,
}

impl AsyncRouter {
    pub async fn listen(url: &str) -> Result<Self, BoxError> {
        let c_url = CString::new(url)?;
        let socket = Rep0Raw::listen(&c_url).await?;
        Ok(Self { socket })
    }

    pub async fn recv(
        &mut self,
    ) -> Result<RouterMessage, BoxError> {
        let msg = self.socket.recv().await?;
        Ok(RouterMessage::from_msg(msg))
    }

    pub async fn send(
        &mut self,
        msg: RouterMessage,
    ) -> Result<(), BoxError> {
        self.socket.send(msg.into_msg()).await.map_err(|(e, _)| e)?;
        Ok(())
    }

    pub fn clone(&self) -> Self {
        Self {
            socket: self.socket.clone(),
        }
    }
}

pub struct RouterMessage(Message);

impl RouterMessage {
    pub fn from_msg(msg: Message) -> Self {
        Self(msg)
    }

    pub fn into_msg(self) -> Message {
        self.0
    }

    pub fn identity(&self) -> Option<&[u8]> {
        self.0.header().get(..4)
    }

    pub fn header(&self) -> &[u8] {
        self.0.header()
    }
}

pub struct DealerMessage(Message);

impl DealerMessage {
    fn from_msg(msg: Message) -> Self {
        Self(msg)
    }
}

impl std::ops::Deref for DealerMessage {
    type Target = Message;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// --- Transport Implementations ---

pub struct ClientTransport {
    send_tx: mpsc::Sender<Bytes>,
    recv_rx: Mutex<mpsc::Receiver<Bytes>>,
}

impl ClientTransport {
    pub async fn new(url: &str) -> Result<Self, BoxError> {
        let mut dealer = AsyncDealer::dial_with_retry(url).await?;
        let socket_rx = dealer.socket.clone();

        let (send_tx, mut send_rx) = mpsc::channel::<Bytes>(256);
        let (recv_tx, recv_rx) = mpsc::channel::<Bytes>(256);

        tokio::spawn(async move {
            while let Some(body) = send_rx.recv().await {
                let req_id = match dealer.send(&body).await {
                    Ok(id) => id,
                    Err(_) => break,
                };

                if cfg!(test) {
                    let stream_id = parse_stream_id(&body).unwrap_or(0);
                    println!(
                        "CLIENT SEND stream_id={} req_id=0x{:08x} len={}",
                        stream_id,
                        req_id,
                        body.len()
                    );
                }
            }
        });

        tokio::spawn(async move {
            let mut socket_rx = socket_rx;
            loop {
                let msg = match socket_rx.recv().await {
                    Ok(m) => m,
                    Err(_) => break,
                };
                let bytes = Bytes::copy_from_slice(msg.as_slice());
                if recv_tx.send(bytes).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            send_tx,
            recv_rx: Mutex::new(recv_rx),
        })
    }

    pub async fn new_with_retry(
        url: &str,
        max_retries: u32,
        retry_delay: Duration,
    ) -> Result<Self, BoxError> {
        let mut dealer = AsyncDealer::dial_with_retry_params(url, max_retries, retry_delay).await?;
        let socket_rx = dealer.socket.clone();

        let (send_tx, mut send_rx) = mpsc::channel::<Bytes>(256);
        let (recv_tx, recv_rx) = mpsc::channel::<Bytes>(256);

        tokio::spawn(async move {
            while let Some(body) = send_rx.recv().await {
                if dealer.send(&body).await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            let mut socket_rx = socket_rx;
            loop {
                let msg = match socket_rx.recv().await {
                    Ok(m) => m,
                    Err(_) => break,
                };
                let bytes = Bytes::copy_from_slice(msg.as_slice());
                if recv_tx.send(bytes).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            send_tx,
            recv_rx: Mutex::new(recv_rx),
        })
    }
}

#[async_trait]
impl Transport for ClientTransport {
    async fn send(&self, data: Bytes) -> Result<(), BoxError> {
        self.send_tx
            .send(data)
            .await
            .map_err(|_| "Transport send channel closed")?;
        Ok(())
    }

    async fn recv(&self) -> Result<Bytes, BoxError> {
        let mut rx = self.recv_rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| "Transport recv channel closed".into())
    }
}

pub struct InboundFrame {
    pub header: Bytes,
    pub body: Bytes,
}

pub struct ServerTransport {
    send_tx: mpsc::Sender<Message>,
    rx: Mutex<mpsc::Receiver<InboundFrame>>,
    routing: Mutex<std::collections::HashMap<u32, Bytes>>,
}

impl ServerTransport {
    pub fn new(router: AsyncRouter, rx: mpsc::Receiver<InboundFrame>) -> Self {
        let (send_tx, mut send_rx) = mpsc::channel::<Message>(256);
        let mut router = router;

        tokio::spawn(async move {
            while let Some(msg) = send_rx.recv().await {
                if router.send(RouterMessage::from_msg(msg)).await.is_err() {
                    break;
                }
            }
        });

        Self {
            send_tx,
            rx: Mutex::new(rx),
            routing: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

fn parse_stream_id(data: &[u8]) -> Option<u32> {
    if data.len() < 4 {
        return None;
    }
    let mut b = [0u8; 4];
    b.copy_from_slice(&data[..4]);
    Some(u32::from_be_bytes(b))
}

#[async_trait]
impl Transport for ServerTransport {
    async fn send(&self, data: Bytes) -> Result<(), BoxError> {
        let stream_id = parse_stream_id(&data).ok_or("Packet too short")?;
        let header = {
            let routing = self.routing.lock().await;
            routing.get(&stream_id).cloned()
        }
        .ok_or("Missing routing header for stream")?;
        #[cfg(test)]
        {
            let req_id = if header.len() >= 4 {
                let mut b = [0u8; 4];
                b.copy_from_slice(&header[header.len() - 4..]);
                u32::from_be_bytes(b)
            } else {
                0
            };
            println!(
                "SERVER SEND stream_id={} req_id=0x{:08x} header_len={} len={}",
                stream_id,
                req_id,
                header.len(),
                data.len()
            );
        }

        let mut msg = Message::with_capacity(data.len());
        msg.write_all(&data)?;
        msg.write_header(&header)?;

        self.send_tx
            .send(msg)
            .await
            .map_err(|_| "Transport send channel closed")?;
        Ok(())
    }

    async fn recv(&self) -> Result<Bytes, BoxError> {
        let mut rx = self.rx.lock().await;
        match rx.recv().await {
            Some(frame) => {
                let _header_len = frame.header.len();
                if let Some(stream_id) = parse_stream_id(&frame.body) {
                    let mut routing = self.routing.lock().await;
                    routing.insert(stream_id, frame.header);
                }
                #[cfg(test)]
                {
                    let stream_id = parse_stream_id(&frame.body).unwrap_or(0);
                    println!(
                        "SERVER RECV stream_id={} header_len={} body_len={}",
                        stream_id,
                        _header_len,
                        frame.body.len()
                    );
                }
                Ok(frame.body)
            }
            None => Err("Channel closed".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dealer_router_smoke() -> Result<(), BoxError> {
        let url = format!("inproc://forpc_transport_smoke_{}", std::process::id());

        tokio::spawn({
            let url = url.clone();
            async move {
                let mut router = AsyncRouter::listen(&url).await?;
                let msg = router.recv().await?;
                let header = Bytes::copy_from_slice(msg.header());

                let mut out = Message::with_capacity(64);
                write!(&mut out, "pong")?;
                out.write_header(&header)?;
                router.send(RouterMessage::from_msg(out)).await?;
                Ok::<(), BoxError>(())
            }
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut dealer = AsyncDealer::dial(&url).await?;
        dealer.send(b"ping").await?;
        let reply = dealer.recv().await?;
        assert_eq!(reply.as_slice(), b"pong");
        Ok(())
    }
}
