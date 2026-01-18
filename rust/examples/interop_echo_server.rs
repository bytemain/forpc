use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::Bytes;
use fory::ForyObject;
use mini_rpc::{RpcPeer};
use mini_rpc::transport::nng::{AsyncRouter, InboundFrame, ServerTransport};
use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;

fn hex_prefix(data: &[u8], limit: usize) -> String {
    let n = std::cmp::min(limit, data.len());
    let mut s = String::new();
    for (i, b) in data[..n].iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[derive(ForyObject, Debug, Clone)]
struct EchoRequest {
    data: String,
}

#[derive(ForyObject, Debug, Clone)]
struct EchoResponse {
    result: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "tcp://127.0.0.1:24000".to_string());

    println!("interop_echo_server: bind {}", url);
    let router = AsyncRouter::listen(&url).await?;
    println!("interop_echo_server: listening");

    let (accept_tx, mut accept_rx) = mpsc::channel::<Arc<RpcPeer>>(100);
    let peers: Arc<Mutex<HashMap<Vec<u8>, mpsc::Sender<InboundFrame>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let router_clone = router.clone();
    let peers_clone = peers.clone();
    let print_ctr = Arc::new(AtomicUsize::new(0));
    let print_ctr_clone = print_ctr.clone();

    tokio::spawn(async move {
        let mut router = router_clone;
        loop {
            match router.recv().await {
                Ok(msg) => {
                    let identity = msg.identity().unwrap_or(&[]).to_vec();
                    if identity.is_empty() {
                        continue;
                    }

                    let header = Bytes::copy_from_slice(msg.header());
                    let mut body = Bytes::copy_from_slice(msg.into_msg().as_slice());

                    if print_ctr_clone.fetch_add(1, Ordering::Relaxed) < 10 {
                        println!("interop_echo_server: raw body: {}", hex_prefix(&body, 24));
                    }

                    if body.len() >= 9 {
                        let kind = body[4];
                        if kind > 2 && body[8] <= 2 {
                            body = body.slice(4..);
                        }
                    }

                    if print_ctr_clone.load(Ordering::Relaxed) <= 10 {
                        println!("interop_echo_server: body: {}", hex_prefix(&body, 24));
                    }

                    let frame = InboundFrame { header, body };

                    let mut peers_map = peers_clone.lock().await;
                    if let Some(tx) = peers_map.get(&identity) {
                        if tx.send(frame).await.is_err() {
                            peers_map.remove(&identity);
                        }
                    } else {
                        let (tx, rx) = mpsc::channel(100);
                        let transport = ServerTransport::new(router.clone(), rx);
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

    while let Some(peer) = accept_rx.recv().await {
        println!("interop_echo_server: accepted");

        peer.register_type_by_namespace::<EchoRequest>("mini_rpc.it", "EchoRequest")
            .await?;
        peer.register_type_by_namespace::<EchoResponse>("mini_rpc.it", "EchoResponse")
            .await?;

        peer.register_unary("Test/Echo", |req: EchoRequest, _meta, _peer| async move {
            Ok(EchoResponse { result: req.data })
        })
        .await;

        tokio::spawn(async move {
            let r = peer.serve().await;
            if let Err(e) = r {
                eprintln!("interop_echo_server: serve error: {}", e);
            }
        });
    }

    Ok(())
}
