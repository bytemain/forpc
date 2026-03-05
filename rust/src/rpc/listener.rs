use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use bytes::Bytes;
use std::collections::HashMap;
use crate::transport::nng::{AsyncRouter, InboundFrame, ServerTransport};
use super::peer::RpcPeer;
use super::protocol::StatusCode;
use super::error::{RpcError, RpcResult};

pub struct RpcListener {
    accept_rx: Mutex<mpsc::Receiver<Arc<RpcPeer>>>,
}

impl RpcListener {
    pub async fn bind(url: &str) -> RpcResult<Self> {
        let router = AsyncRouter::listen(url)
            .await
            .map_err(|e| RpcError::new(StatusCode::Unavailable as i32, e.to_string()))?;

        let (accept_tx, accept_rx) = mpsc::channel(100);
        let peers: Arc<Mutex<HashMap<Vec<u8>, mpsc::Sender<InboundFrame>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let mut router_clone = router.clone();
        let peers_clone = peers.clone();

        tokio::spawn(async move {
            loop {
                match router_clone.recv().await {
                    Ok(msg) => {
                        let identity = msg.identity().unwrap_or(&[]).to_vec();
                        if identity.is_empty() {
                            continue;
                        }

                        let header = Bytes::copy_from_slice(msg.header());
                        let payload = Bytes::copy_from_slice(msg.into_msg().as_slice());
                        let frame = InboundFrame { header, body: payload };

                        let mut peers_map = peers_clone.lock().await;

                        if let Some(tx) = peers_map.get(&identity) {
                            if tx.send(frame).await.is_err() {
                                peers_map.remove(&identity);
                            }
                        } else {
                            let (tx, rx) = mpsc::channel(100);
                            let transport = ServerTransport::new(router_clone.clone(), rx);
                            let peer = Arc::new(RpcPeer::new(transport, false));

                            if accept_tx.send(peer).await.is_ok() {
                                peers_map.insert(identity, tx.clone());
                                let _ = tx.send(frame).await;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            accept_rx: Mutex::new(accept_rx),
        })
    }
    
    pub async fn accept(&self) -> RpcResult<Arc<RpcPeer>> {
        let mut rx = self.accept_rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| RpcError::new(StatusCode::Unavailable as i32, "Listener closed"))
    }
}
