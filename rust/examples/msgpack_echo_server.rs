use std::env;
use std::sync::Arc;

use bytes::Bytes;
use forpc::{Request, Response, RpcListener, RpcPeer, StatusCode};

#[derive(serde::Serialize, serde::Deserialize)]
struct EchoMsg {
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "tcp://127.0.0.1:24010".to_string());

    let listener = RpcListener::bind(&url).await?;
    let peer = listener.accept().await?;

    peer.register("MsgPack/Echo", |req: Request, _peer: Arc<RpcPeer>| async move {
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
            return Response::error_with_code(StatusCode::InvalidArgument, "Missing payload");
        }

        let msg: EchoMsg = match rmp_serde::from_slice(&payload) {
            Ok(m) => m,
            Err(e) => {
                return Response::error_with_code(
                    StatusCode::InvalidArgument,
                    &format!("Failed to decode MessagePack: {e}"),
                );
            }
        };

        let reply = EchoMsg {
            message: msg.message,
        };
        match rmp_serde::to_vec_named(&reply) {
            Ok(data) => Response::ok(Bytes::from(data)),
            Err(e) => Response::error_with_code(
                StatusCode::Internal,
                &format!("Failed to encode MessagePack: {e}"),
            ),
        }
    })
    .await;

    peer.serve().await?;
    Ok(())
}
