use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use forpc::{Request, Response, RpcListener, RpcPeer, Status, StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(prost::Message, Clone)]
struct TcpChunk {
    #[prost(bytes = "vec", tag = "1")]
    data: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let mut args = env::args().skip(1);
    let listen_url = args
        .next()
        .unwrap_or_else(|| "tcp://0.0.0.0:4000".to_string());

    let listener = RpcListener::bind(&listen_url).await?;
    loop {
        let peer = listener.accept().await?;

        peer.register("Tunnel/Tcp", tunnel_tcp).await;

        tokio::spawn(async move {
            let _ = peer.serve().await;
        });
    }
}

fn resp_ok() -> Response {
    Response {
        metadata: HashMap::new(),
        payload: None,
        status: Status::ok(),
    }
}

async fn tunnel_tcp(req: Request, peer: Arc<RpcPeer>) -> Response {
    let Some(mut rx) = req.stream else {
        return Response::error_with_code(StatusCode::InvalidArgument, "missing stream");
    };
    let dst = match req.metadata.get("dst") {
        Some(v) if !v.is_empty() => v.clone(),
        _ => return Response::error_with_code(StatusCode::InvalidArgument, "missing dst"),
    };

    let socket = match tokio::net::TcpStream::connect(dst).await {
        Ok(s) => s,
        Err(e) => return Response::error_with_code(StatusCode::Unavailable, e.to_string()),
    };

    let (mut r, mut w) = socket.into_split();
    let stream_id = req.stream_id;

    let peer_in = peer.clone();
    let t_in = tokio::spawn(async move {
        while let Some(pkt) = rx.recv().await {
            match pkt.kind {
                forpc::rpc::protocol::frame_kind::DATA => {
                    let chunk: TcpChunk = peer_in.user_deserialize(&pkt.payload).await?;
                    if !chunk.data.is_empty() {
                        w.write_all(&chunk.data).await.map_err(|e| {
                            forpc::RpcError::new(StatusCode::Unavailable, e.to_string())
                        })?;
                    }
                }
                forpc::rpc::protocol::frame_kind::TRAILERS => {
                    let _ = w.shutdown().await;
                    break;
                }
                _ => {}
            }
        }
        Ok::<(), forpc::RpcError>(())
    });

    let peer_out = peer.clone();
    let t_out = tokio::spawn(async move {
        let mut buf = [0u8; 16 * 1024];
        loop {
            let n = r.read(&mut buf).await.map_err(|e| {
                forpc::RpcError::new(StatusCode::Unavailable, e.to_string())
            })?;
            if n == 0 {
                break;
            }
            let payload = peer_out
                .user_serialize(&TcpChunk {
                    data: buf[..n].to_vec(),
                })
                .await?;
            peer_out.send_stream_data(stream_id, payload).await?;
        }
        Ok::<(), forpc::RpcError>(())
    });

    let r = tokio::select! {
        r = t_in => r,
        r = t_out => r,
    };

    match r {
        Ok(Ok(())) => resp_ok(),
        Ok(Err(e)) => {
            let code = StatusCode::try_from(e.code).unwrap_or(StatusCode::Unknown);
            Response::error_with_code(code, e.message)
        }
        Err(e) => Response::error_with_code(StatusCode::Internal, e.to_string()),
    }
}
