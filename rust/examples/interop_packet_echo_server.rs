use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::{mpsc, Mutex};

use forpc::rpc::protocol::Packet;
use forpc::transport::nng::{AsyncRouter, InboundFrame, ServerTransport, Transport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "tcp://127.0.0.1:24001".to_string());

    println!("interop_packet_echo_server: bind {}", url);
    let mut router = AsyncRouter::listen(&url).await?;
    println!("interop_packet_echo_server: listening");

    let peers: Arc<Mutex<HashMap<Vec<u8>, mpsc::Sender<InboundFrame>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        let msg = router.recv().await?;
        let identity = msg.identity().unwrap_or(&[]).to_vec();
        if identity.is_empty() {
            continue;
        }

        let header = Bytes::copy_from_slice(msg.header());
        let body = Bytes::copy_from_slice(msg.into_msg().as_slice());
        let frame = InboundFrame { header, body };

        let mut peers_map = peers.lock().await;
        if let Some(tx) = peers_map.get(&identity) {
            if tx.send(frame).await.is_err() {
                peers_map.remove(&identity);
            }
        } else {
            let (tx, rx) = mpsc::channel(100);
            let transport = ServerTransport::new(router.clone(), rx);
            tokio::spawn(async move {
                let _ = echo_loop(transport).await;
            });
            peers_map.insert(identity, tx.clone());
            let _ = tx.send(frame).await;
        }
    }
}

async fn echo_loop(transport: ServerTransport) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let data = transport.recv().await?;
        let packet = match Packet::decode_from_bytes(&data) {
            Ok(p) => p,
            Err(_) => continue,
        };
        transport.send(packet.encode_to_bytes()).await?;
    }
}
