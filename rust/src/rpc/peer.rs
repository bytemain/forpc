use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;
use tokio::sync::{mpsc, Mutex, oneshot, RwLock};
use tokio_util::sync::CancellationToken;
use bytes::Bytes;
use prost::Message;

use super::protocol::{Packet, Call, Status, StatusCode, frame_kind};
use super::error::{RpcError, RpcResult};
use super::stream::BidiStream;
use crate::transport::nng::{ClientTransport, Transport};

// Handler type alias
pub type BoxedHandler = Arc<
    dyn Fn(Request, Arc<RpcPeer>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync
>;

pub struct RpcPeer {
    pub(crate) transport: Arc<dyn Transport>,
    pub(crate) handlers: Arc<RwLock<HashMap<String, BoxedHandler>>>,
    pub(crate) pending_calls: Arc<Mutex<HashMap<u32, PendingCall>>>,
    pub(crate) inbound_streams: Arc<Mutex<HashMap<u32, StreamState>>>,
    pub(crate) inbound_cancels: Arc<Mutex<HashMap<u32, CancellationToken>>>,
    pub(crate) early_inbound: Arc<Mutex<HashMap<u32, Vec<Packet>>>>,
    next_stream_id: AtomicU32,
    is_initiator: bool,
    running: Arc<AtomicBool>,
}

pub(crate) struct PendingCall {
    tx: Option<oneshot::Sender<RpcResult<Bytes>>>,
    stream_tx: Option<mpsc::Sender<Packet>>,
    unary_buffer: Option<Bytes>,
}

pub(crate) struct StreamState {
    tx: mpsc::Sender<Packet>,
}

pub struct Request {
    pub method: String,
    pub metadata: HashMap<String, String>,
    pub payload: Option<Bytes>,
    pub stream: Option<mpsc::Receiver<Packet>>, // Packet contains payload
    pub stream_id: u32,
    pub cancel_token: CancellationToken,
}

pub struct Response {
    pub metadata: HashMap<String, String>,
    pub payload: Option<Bytes>,
    pub status: Status,
}

impl Response {
    pub fn ok(payload: Bytes) -> Self {
        Self {
            metadata: HashMap::new(),
            payload: Some(payload),
            status: Status::ok(),
        }
    }
    
    pub fn error(status: Status) -> Self {
        Self {
            metadata: HashMap::new(),
            payload: None,
            status,
        }
    }
    
    pub fn error_with_code(code: StatusCode, message: impl Into<String>) -> Self {
        Self::error(Status::new(code, message))
    }
}

impl RpcPeer {
    pub async fn connect(url: &str) -> RpcResult<Arc<Self>> {
        let transport = ClientTransport::new(url)
            .await
            .map_err(|e| RpcError::new(StatusCode::Unavailable, e.to_string()))?;
        Ok(Arc::new(Self::new(transport, true)))
    }

    pub async fn connect_with_retry(url: &str, max_retries: u32) -> RpcResult<Arc<Self>> {
        let transport = ClientTransport::new_with_retry(
            url,
            max_retries,
            std::time::Duration::from_millis(100),
        )
        .await
        .map_err(|e| RpcError::new(StatusCode::Unavailable, e.to_string()))?;
        Ok(Arc::new(Self::new(transport, true)))
    }

    pub fn transport_arc(&self) -> Arc<dyn Transport> {
        self.transport.clone()
    }

    pub async fn user_serialize<T>(&self, value: &T) -> RpcResult<Bytes>
    where
        T: Message + Send + Sync + 'static,
    {
        Ok(Bytes::from(value.encode_to_vec()))
    }

    pub async fn user_deserialize<T>(&self, bytes: &Bytes) -> RpcResult<T>
    where
        T: Message + Default + Send + Sync + 'static,
    {
        T::decode(bytes.as_ref())
            .map_err(|e| RpcError::new(StatusCode::Internal, e.to_string()))
    }

    pub async fn send_stream_data(&self, stream_id: u32, payload: Bytes) -> RpcResult<()> {
        self.send_packet(Packet::data(stream_id, payload)).await
    }

    pub async fn send_stream_trailers(&self, stream_id: u32, status: &Status) -> RpcResult<()> {
        let packet = Packet::trailers(stream_id, status);
        self.send_packet(packet).await
    }

    pub fn new(transport: impl Transport + 'static, is_initiator: bool) -> Self {
        Self {
            transport: Arc::new(transport),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            pending_calls: Arc::new(Mutex::new(HashMap::new())),
            inbound_streams: Arc::new(Mutex::new(HashMap::new())),
            inbound_cancels: Arc::new(Mutex::new(HashMap::new())),
            early_inbound: Arc::new(Mutex::new(HashMap::new())),
            next_stream_id: AtomicU32::new(if is_initiator { 1 } else { 2 }),
            is_initiator,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    fn alloc_stream_id(&self) -> u32 {
        self.next_stream_id.fetch_add(2, Ordering::Relaxed)
    }
    
    pub async fn register<F, Fut>(&self, method: &str, handler: F)
    where
        F: Fn(Request, Arc<RpcPeer>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let handler = move |req: Request, peer: Arc<RpcPeer>| -> Pin<Box<dyn Future<Output = Response> + Send>> {
            Box::pin(handler(req, peer))
        };
        
        let mut handlers = self.handlers.write().await;
        handlers.insert(method.to_string(), Arc::new(handler));
    }
    
    pub async fn register_unary<Req, Resp, F, Fut>(&self, method: &str, handler: F)
    where
        Req: Message + Default + Send + Sync + 'static,
        Resp: Message + Default + Send + Sync + 'static,
        F: Fn(Req, HashMap<String, String>, Arc<RpcPeer>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Resp>> + Send + 'static,
    {
        self.register(method, move |req: Request, peer: Arc<RpcPeer>| {
            let handler = handler.clone();
            
            async move {
                let mut payload = Bytes::new();
                if let Some(mut rx) = req.stream {
                    while let Some(packet) = rx.recv().await {
                        if packet.kind == frame_kind::DATA {
                            payload = packet.payload;
                        }
                    }
                } else if let Some(p) = req.payload {
                    payload = p;
                }

                if payload.is_empty() {
                    return Response::error_with_code(StatusCode::InvalidArgument, "Missing payload");
                }
                
                let request: Req = match Req::decode(payload.as_ref()) {
                    Ok(r) => r,
                    Err(e) => return Response::error_with_code(StatusCode::InvalidArgument, format!("Deserialize error: {}", e)),
                };
                
                match handler(request, req.metadata, peer).await {
                    Ok(response) => {
                        Response::ok(Bytes::from(response.encode_to_vec()))
                    }
                    Err(e) => {
                        let code = StatusCode::try_from(e.code).unwrap_or(StatusCode::Unknown);
                        Response::error_with_code(code, e.message)
                    }
                }
            }
        }).await;
    }
    
    pub async fn send_packet(&self, packet: Packet) -> RpcResult<()> {
        let bytes = {
            packet.encode().map_err(|e| RpcError::new(StatusCode::Internal, e.to_string()))?
        };
        self.transport.send(bytes).await.map_err(|e| RpcError::new(StatusCode::Unavailable, e.to_string()))?;
        Ok(())
    }

    async fn insert_pending_call(&self, stream_id: u32, pending_call: PendingCall) {
        let mut pending = self.pending_calls.lock().await;
        pending.insert(stream_id, pending_call);
    }

    pub(crate) async fn remove_pending_call(&self, stream_id: u32) {
        let mut pending = self.pending_calls.lock().await;
        pending.remove(&stream_id);
    }

    async fn send_call_headers(&self, stream_id: u32, call: &Call) -> RpcResult<()> {
        let packet = Packet::headers(stream_id, call);
        self.send_packet(packet).await
    }
    
    pub async fn stream<Req, Resp>(self: &Arc<Self>, method: &str) -> RpcResult<BidiStream<Req, Resp>>
    where Req: Message + Send + Sync + 'static, Resp: Message + Default + Send + Sync + 'static
    {
        self.stream_with_metadata(method, HashMap::new()).await
    }

    pub async fn stream_with_metadata<Req, Resp>(
        self: &Arc<Self>,
        method: &str,
        metadata: HashMap<String, String>,
    ) -> RpcResult<BidiStream<Req, Resp>>
    where
        Req: Message + Send + Sync + 'static,
        Resp: Message + Default + Send + Sync + 'static,
    {
        let stream_id = self.alloc_stream_id();
        let call = Call {
            method: method.to_string(),
            metadata,
        };

        let (tx, rx) = mpsc::channel(32);
        self
            .insert_pending_call(
                stream_id,
                PendingCall {
                    tx: None,
                    stream_tx: Some(tx),
                    unary_buffer: None,
                },
            )
            .await;

        if let Err(e) = self.send_call_headers(stream_id, &call).await {
            self.remove_pending_call(stream_id).await;
            return Err(e);
        }

        Ok(BidiStream {
            stream_id,
            peer: self.clone(),
            recv_rx: rx,
            _marker: std::marker::PhantomData,
        })
    }
    
    pub async fn call<Req, Resp>(&self, method: &str, request: Req) -> RpcResult<Resp>
    where Req: Message + Send + Sync + 'static, Resp: Message + Default + Send + Sync + 'static
    {
        self.call_with_metadata(method, request, HashMap::new()).await
    }

    pub async fn call_with_metadata<Req, Resp>(
        &self,
        method: &str,
        request: Req,
        metadata: HashMap<String, String>,
    ) -> RpcResult<Resp>
    where
        Req: Message + Send + Sync + 'static,
        Resp: Message + Default + Send + Sync + 'static,
    {
        let payload = self.user_serialize(&request).await?;
        let resp_bytes = self
            .unary_exchange_raw_with_metadata(method, payload, metadata)
            .await?;
        self.user_deserialize(&resp_bytes).await
    }

    pub async fn call_raw(&self, method: &str, payload: Bytes) -> RpcResult<Bytes> {
        self.call_raw_with_metadata(method, payload, HashMap::new()).await
    }

    pub async fn call_raw_with_metadata(
        &self,
        method: &str,
        payload: Bytes,
        metadata: HashMap<String, String>,
    ) -> RpcResult<Bytes> {
        self.unary_exchange_raw_with_metadata(method, payload, metadata)
            .await
    }

    async fn unary_exchange_raw_with_metadata(
        &self,
        method: &str,
        payload: Bytes,
        metadata: HashMap<String, String>,
    ) -> RpcResult<Bytes> {
        let stream_id = self.alloc_stream_id();
        let call = Call {
            method: method.to_string(),
            metadata,
        };

        let timeout_duration = call.timeout_ms().map(std::time::Duration::from_millis);

        let (tx, rx) = oneshot::channel();
        self
            .insert_pending_call(
                stream_id,
                PendingCall {
                    tx: Some(tx),
                    stream_tx: None,
                    unary_buffer: None,
                },
            )
            .await;

        if let Err(e) = self.send_call_headers(stream_id, &call).await {
            self.remove_pending_call(stream_id).await;
            return Err(e);
        }

        if let Err(e) = self.send_packet(Packet::data(stream_id, payload)).await {
            self.remove_pending_call(stream_id).await;
            return Err(e);
        }

        if let Err(e) = self.send_stream_trailers(stream_id, &Status::ok()).await {
            self.remove_pending_call(stream_id).await;
            return Err(e);
        }

        if let Some(duration) = timeout_duration {
            match tokio::time::timeout(duration, rx).await {
                Ok(result) => result
                    .map_err(|_| RpcError::new(StatusCode::Cancelled, "Call cancelled"))?,
                Err(_) => {
                    self.remove_pending_call(stream_id).await;
                    let _ = self.send_packet(Packet::rst_stream(stream_id, StatusCode::Cancelled as u32)).await;
                    Err(RpcError::new(StatusCode::DeadlineExceeded, "Deadline exceeded"))
                }
            }
        } else {
            rx.await
                .map_err(|_| RpcError::new(StatusCode::Cancelled, "Call cancelled"))?
        }
    }
    
    pub async fn serve(self: &Arc<Self>) -> RpcResult<()> {
        while self.running.load(Ordering::Relaxed) {
            let bytes = match self.transport.recv().await {
                Ok(b) => b,
                Err(e) => {
                    if self.running.load(Ordering::Relaxed) {
                        return Err(RpcError::new(StatusCode::Unavailable, e.to_string()));
                    }
                    break;
                }
            };
            
            let packet = {
                Packet::decode(bytes).map_err(|e| RpcError::new(StatusCode::Internal, e.to_string()))?
            };

            if packet.stream_id == 0 {
                continue;
            }
            
            let is_inbound = if self.is_initiator {
                packet.stream_id % 2 == 0
            } else {
                packet.stream_id % 2 == 1
            };
            
            let result = if is_inbound {
                self.handle_inbound(packet).await
            } else {
                self.handle_outbound(packet).await
            };
            if let Err(e) = result {
                eprintln!("{}", e);
            }
        }
        Ok(())
    }
    
    async fn handle_inbound(self: &Arc<Self>, packet: Packet) -> RpcResult<()> {
        let stream_id = packet.stream_id;
        match packet.kind {
            frame_kind::HEADERS => {
                let call: Call = Call::decode(packet.payload.as_ref())
                    .map_err(|e| RpcError::new(StatusCode::Internal, e.to_string()))?;
                
                let handler = {
                    let h = self.handlers.read().await;
                    h.get(&call.method).cloned()
                };

                if let Some(handler) = handler {
                    let cancel_token = CancellationToken::new();
                    let (tx, rx) = mpsc::channel(32);
                    {
                        let mut streams = self.inbound_streams.lock().await;
                        streams.insert(stream_id, StreamState { tx: tx.clone() });
                    }
                    {
                        let mut cancels = self.inbound_cancels.lock().await;
                        cancels.insert(stream_id, cancel_token.clone());
                    }

                    if let Some(buffered) = {
                        let mut early = self.early_inbound.lock().await;
                        early.remove(&stream_id)
                    } {
                        for pkt in buffered {
                            if pkt.kind == frame_kind::TRAILERS {
                                let mut streams = self.inbound_streams.lock().await;
                                if let Some(state) = streams.remove(&stream_id) {
                                    let _ = state.tx.send(pkt).await;
                                }
                                break;
                            } else {
                                let _ = tx.send(pkt).await;
                            }
                        }
                    }
                    
                    let req = Request {
                        method: call.method,
                        metadata: call.metadata,
                        payload: None,
                        stream: Some(rx),
                        stream_id,
                        cancel_token: cancel_token.clone(),
                    };
                    
                    let peer_clone = self.clone();
                    tokio::spawn(async move {
                        let response = handler(req, peer_clone.clone()).await;
                        // Clean up cancel token after handler completes
                        {
                            let mut cancels = peer_clone.inbound_cancels.lock().await;
                            cancels.remove(&stream_id);
                        }
                        if let Err(e) = peer_clone.send_response(stream_id, response).await {
                             eprintln!("Failed to send response: {}", e);
                        }
                    });
                } else {
                    let _ = self.send_response(stream_id, Response::error_with_code(StatusCode::Unimplemented, format!("Method {} not found", call.method))).await;
                }
            }
            frame_kind::RST_STREAM => {
                {
                    let mut cancels = self.inbound_cancels.lock().await;
                    if let Some(token) = cancels.remove(&stream_id) {
                        token.cancel();
                    }
                }
                {
                    let mut streams = self.inbound_streams.lock().await;
                    streams.remove(&stream_id);
                }
                {
                    let mut early = self.early_inbound.lock().await;
                    early.remove(&stream_id);
                }
            }
            frame_kind::DATA | frame_kind::TRAILERS => {
                let mut packet_opt = Some(packet);
                let delivered = {
                    let mut streams = self.inbound_streams.lock().await;
                    let packet = packet_opt.as_ref().unwrap();
                    if packet.kind == frame_kind::TRAILERS {
                        if let Some(state) = streams.remove(&stream_id) {
                            let _ = state.tx.send(packet_opt.take().unwrap()).await;
                            true
                        } else {
                            false
                        }
                    } else {
                        if let Some(state) = streams.get_mut(&stream_id) {
                            let _ = state.tx.send(packet_opt.take().unwrap()).await;
                            true
                        } else {
                            false
                        }
                    }
                };

                if !delivered {
                    let mut early = self.early_inbound.lock().await;
                    early.entry(stream_id).or_default().push(packet_opt.take().unwrap());
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn send_response(&self, stream_id: u32, response: Response) -> RpcResult<()> {
        if let Some(payload) = response.payload {
            self.send_packet(Packet::data(stream_id, payload)).await?;
        }
        let packet = Packet::trailers(stream_id, &response.status);
        self.send_packet(packet).await
    }

    async fn handle_outbound(self: &Arc<Self>, packet: Packet) -> RpcResult<()> {
        let stream_id = packet.stream_id;
        match packet.kind {
            frame_kind::DATA => {
                 let mut pending = self.pending_calls.lock().await;
                 if let Some(call) = pending.get_mut(&stream_id) {
                     if let Some(tx) = &call.stream_tx {
                         let _ = tx.send(packet).await;
                     } else if call.tx.is_some() {
                         call.unary_buffer = Some(packet.payload);
                     }
                 }
            }
            frame_kind::TRAILERS => {
                let mut pending = self.pending_calls.lock().await;
                if let Some(mut call) = pending.remove(&stream_id) {
                     let status: Status = match Status::decode(packet.payload.as_ref()) {
                         Ok(s) => s,
                         Err(e) => Status::internal(e.to_string()),
                     };
                     
                     if let Some(tx) = call.tx {
                         if status.is_ok() {
                             let payload = call.unary_buffer.take().unwrap_or_else(Bytes::new);
                             let _ = tx.send(Ok(payload));
                         } else {
                             let _ = tx.send(Err(RpcError::from_status(status)));
                         }
                     }
                     if let Some(tx) = call.stream_tx {
                         let _ = tx.send(packet).await; 
                     }
                }
            }
            frame_kind::RST_STREAM => {
                let mut pending = self.pending_calls.lock().await;
                if let Some(call) = pending.remove(&stream_id) {
                    if let Some(tx) = call.tx {
                        let _ = tx.send(Err(RpcError::new(StatusCode::Cancelled, "Stream reset by peer")));
                    }
                    if let Some(tx) = call.stream_tx {
                        drop(tx);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tokio::task::JoinHandle;
    use crate::BoxError;

    struct ChannelTransport {
        send_tx: mpsc::Sender<Bytes>,
        recv_rx: Mutex<mpsc::Receiver<Bytes>>,
    }

    impl ChannelTransport {
        fn pair(buffer: usize) -> (Self, Self) {
            let (a_to_b_tx, b_rx) = mpsc::channel::<Bytes>(buffer);
            let (b_to_a_tx, a_rx) = mpsc::channel::<Bytes>(buffer);
            (
                Self {
                    send_tx: a_to_b_tx,
                    recv_rx: Mutex::new(a_rx),
                },
                Self {
                    send_tx: b_to_a_tx,
                    recv_rx: Mutex::new(b_rx),
                },
            )
        }
    }

    #[async_trait]
    impl Transport for ChannelTransport {
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

    #[derive(prost::Message, Clone, PartialEq)]
    struct TestRequest {
        #[prost(string, tag = "1")]
        data: String,
    }

    #[derive(prost::Message, Clone, PartialEq)]
    struct TestResponse {
        #[prost(string, tag = "1")]
        result: String,
    }

    async fn spawn_peer(peer: Arc<RpcPeer>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let _ = peer.serve().await;
        })
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unary_call_smoke_channel_transport() {
        let (ta, tb) = ChannelTransport::pair(256);
        let a = Arc::new(RpcPeer::new(ta, true));
        let b = Arc::new(RpcPeer::new(tb, false));

        b.register_unary("Test/Echo", |req: TestRequest, _meta, _peer| async move {
            Ok(TestResponse { result: req.data })
        })
        .await;

        let _a_task = spawn_peer(a.clone()).await;
        let _b_task = spawn_peer(b.clone()).await;

        let resp: TestResponse = a
            .call("Test/Echo", TestRequest { data: "Hello".into() })
            .await
            .unwrap();
        assert_eq!(resp, TestResponse { result: "Hello".into() });
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unary_call_with_metadata_smoke() {
        let (ta, tb) = ChannelTransport::pair(256);
        let a = Arc::new(RpcPeer::new(ta, true));
        let b = Arc::new(RpcPeer::new(tb, false));

        b.register_unary("Test/EchoMeta", |req: TestRequest, meta, _peer| async move {
            let ok = meta.get("k").map(|v| v.as_str()) == Some("v");
            if !ok {
                return Err(RpcError::new(StatusCode::InvalidArgument, "bad metadata"));
            }
            Ok(TestResponse { result: req.data })
        })
        .await;

        let _a_task = spawn_peer(a.clone()).await;
        let _b_task = spawn_peer(b.clone()).await;

        let mut meta = HashMap::new();
        meta.insert("k".into(), "v".into());
        let resp: TestResponse = a
            .call_with_metadata("Test/EchoMeta", TestRequest { data: "Hello".into() }, meta)
            .await
            .unwrap();
        assert_eq!(resp, TestResponse { result: "Hello".into() });
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stream_id_zero_is_ignored() {
        let (ta, tb) = ChannelTransport::pair(256);
        let a = Arc::new(RpcPeer::new(ta, true));
        let b = Arc::new(RpcPeer::new(tb, false));

        b.register_unary("Test/Echo", |req: TestRequest, _meta, _peer| async move {
            Ok(TestResponse { result: req.data })
        })
        .await;

        let _a_task = spawn_peer(a.clone()).await;
        let _b_task = spawn_peer(b.clone()).await;

        let junk = Packet::data(0, Bytes::from_static(b"junk")).encode().unwrap();
        a.transport.send(junk).await.unwrap();

        let resp: TestResponse = a
            .call("Test/Echo", TestRequest { data: "Ok".into() })
            .await
            .unwrap();
        assert_eq!(resp, TestResponse { result: "Ok".into() });
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unary_call_deadline_exceeded() {
        let (ta, tb) = ChannelTransport::pair(256);
        let a = Arc::new(RpcPeer::new(ta, true));
        let b = Arc::new(RpcPeer::new(tb, false));

        b.register_unary("Test/Slow", |req: TestRequest, _meta, _peer| async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            Ok(TestResponse { result: req.data })
        })
        .await;

        let _a_task = spawn_peer(a.clone()).await;
        let _b_task = spawn_peer(b.clone()).await;

        let mut meta = HashMap::new();
        meta.insert(":timeout".into(), "100".into());
        let err = a
            .call_with_metadata::<TestRequest, TestResponse>(
                "Test/Slow",
                TestRequest { data: "Hello".into() },
                meta,
            )
            .await
            .unwrap_err();

        assert_eq!(err.code, StatusCode::DeadlineExceeded as i32);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unary_call_completes_before_timeout() {
        let (ta, tb) = ChannelTransport::pair(256);
        let a = Arc::new(RpcPeer::new(ta, true));
        let b = Arc::new(RpcPeer::new(tb, false));

        b.register_unary("Test/Echo", |req: TestRequest, _meta, _peer| async move {
            Ok(TestResponse { result: req.data })
        })
        .await;

        let _a_task = spawn_peer(a.clone()).await;
        let _b_task = spawn_peer(b.clone()).await;

        let mut meta = HashMap::new();
        meta.insert(":timeout".into(), "5000".into());
        let resp: TestResponse = a
            .call_with_metadata("Test/Echo", TestRequest { data: "Fast".into() }, meta)
            .await
            .unwrap();
        assert_eq!(resp, TestResponse { result: "Fast".into() });
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rst_stream_cancels_server_handler() {
        let (ta, tb) = ChannelTransport::pair(256);
        let a = Arc::new(RpcPeer::new(ta, true));
        let b = Arc::new(RpcPeer::new(tb, false));

        // Register a slow handler that checks for cancellation
        let cancel_observed = Arc::new(AtomicBool::new(false));
        let cancel_observed_clone = cancel_observed.clone();

        b.register("Test/SlowCancel", move |req: Request, _peer| {
            let observed = cancel_observed_clone.clone();
            Box::pin(async move {
                // Wait for either cancellation or a long time
                tokio::select! {
                    _ = req.cancel_token.cancelled() => {
                        observed.store(true, Ordering::SeqCst);
                        Response::error_with_code(StatusCode::Cancelled, "Cancelled")
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                        Response::ok(Bytes::from_static(b"done"))
                    }
                }
            })
        }).await;

        let _a_task = spawn_peer(a.clone()).await;
        let _b_task = spawn_peer(b.clone()).await;

        // Client sends a call with a short timeout, which will trigger RST_STREAM
        let mut meta = HashMap::new();
        meta.insert(":timeout".into(), "100".into());
        let err = a
            .call_raw_with_metadata("Test/SlowCancel", Bytes::from_static(b"test"), meta)
            .await
            .unwrap_err();
        assert_eq!(err.code, StatusCode::DeadlineExceeded as i32);

        // Give time for the RST_STREAM to propagate to the server
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert!(cancel_observed.load(Ordering::SeqCst), "Server handler should have observed cancellation");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rst_stream_outbound_resolves_pending_call() {
        let (ta, tb) = ChannelTransport::pair(256);
        let a = Arc::new(RpcPeer::new(ta, true));
        let b = Arc::new(RpcPeer::new(tb, false));

        let _a_task = spawn_peer(a.clone()).await;

        // Manually send RST_STREAM from the "server" side to cancel a pending call
        let stream_id = 1u32; // First stream ID for initiator (odd = outbound for a)
        // Insert a pending call on a's side
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = a.pending_calls.lock().await;
            pending.insert(stream_id, PendingCall {
                tx: Some(tx),
                stream_tx: None,
                unary_buffer: None,
            });
        }

        // Send RST_STREAM from b (server) → arrives at a (client)
        let rst = Packet::rst_stream(stream_id, StatusCode::Cancelled as u32);
        let encoded = rst.encode().unwrap();
        b.transport.send(encoded).await.unwrap();

        // The pending call should be resolved with CANCELLED error
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), rx).await
            .expect("Should resolve within timeout")
            .expect("Channel should not be dropped");
        let err = result.unwrap_err();
        assert_eq!(err.code, StatusCode::Cancelled as i32);
    }

    #[test]
    fn rst_stream_packet_roundtrip() {
        let p = Packet::rst_stream(42, StatusCode::Cancelled as u32);
        let encoded = p.encode().unwrap();
        let decoded = Packet::decode(encoded).unwrap();
        assert_eq!(decoded.stream_id, 42);
        assert_eq!(decoded.kind, frame_kind::RST_STREAM);
        assert_eq!(decoded.payload.len(), 4);
        // Verify error code
        let error_code = u32::from_be_bytes(decoded.payload[..4].try_into().unwrap());
        assert_eq!(error_code, StatusCode::Cancelled as u32);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn bidi_stream_cancel_sends_rst_stream() {
        let (ta, tb) = ChannelTransport::pair(256);
        let a = Arc::new(RpcPeer::new(ta, true));
        let b = Arc::new(RpcPeer::new(tb, false));

        // Register a handler that waits for cancellation via cancel_token
        let cancel_observed = Arc::new(AtomicBool::new(false));
        let cancel_observed_clone = cancel_observed.clone();

        b.register("Chat/Stream", move |req: Request, _peer| {
            let observed = cancel_observed_clone.clone();
            Box::pin(async move {
                tokio::select! {
                    _ = req.cancel_token.cancelled() => {
                        observed.store(true, Ordering::SeqCst);
                        Response::error_with_code(StatusCode::Cancelled, "Cancelled")
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                        Response::ok(Bytes::from_static(b"done"))
                    }
                }
            })
        }).await;

        let _a_task = spawn_peer(a.clone()).await;
        let _b_task = spawn_peer(b.clone()).await;

        // Open a stream and then cancel it
        let mut stream = a.stream::<TestRequest, TestResponse>("Chat/Stream").await.unwrap();
        stream.send(TestRequest { data: "hello".into() }).await.unwrap();

        // Cancel the stream — this sends RST_STREAM
        stream.cancel().await.unwrap();

        // Wait for cancellation to propagate
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert!(cancel_observed.load(Ordering::SeqCst), "Server handler should have observed stream cancellation");
    }
}
