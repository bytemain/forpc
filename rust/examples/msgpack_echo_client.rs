use std::env;

use bytes::Bytes;
use forpc::RpcPeer;

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
    let method = args.next().unwrap_or_else(|| "MsgPack/Echo".to_string());
    let msg = args.next().unwrap_or_else(|| "Hello".to_string());

    let peer = RpcPeer::connect_with_retry(&url, 50).await?;

    let peer_clone = peer.clone();
    tokio::spawn(async move {
        let _ = peer_clone.serve().await;
    });

    let req = EchoMsg { message: msg };
    let payload = rmp_serde::to_vec(&req)?;

    let resp = peer.call_raw(&method, Bytes::from(payload)).await?;

    let reply: EchoMsg = rmp_serde::from_slice(&resp)?;
    println!("reply: {}", reply.message);
    Ok(())
}
