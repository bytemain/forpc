use std::env;
use std::sync::Arc;

use bytes::Bytes;
use mini_rpc::{Request, Response, RpcListener, RpcPeer, StatusCode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "tcp://127.0.0.1:24002".to_string());

    let listener = RpcListener::bind(&url).await?;
    let peer = listener.accept().await?;

    peer.register("Raw/Echo", |req: Request, _peer: Arc<RpcPeer>| async move {
        let mut payload = Bytes::new();
        if let Some(mut rx) = req.stream {
            while let Some(packet) = rx.recv().await {
                if packet.kind == 1 {
                    payload = packet.payload;
                } else if packet.kind == 2 {
                    break;
                }
            }
        }
        if payload.is_empty() {
            return Response::error_with_code(StatusCode::INVALID_ARGUMENT, "Missing payload");
        }
        Response::ok(payload)
    })
    .await;

    peer.serve().await?;
    Ok(())
}

