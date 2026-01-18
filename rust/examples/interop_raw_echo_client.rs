use std::env;

use bytes::Bytes;
use mini_rpc::RpcPeer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "tcp://127.0.0.1:24002".to_string());
    let method = args.next().unwrap_or_else(|| "Raw/Echo".to_string());
    let msg = args.next().unwrap_or_else(|| "Hello".to_string());

    let peer = RpcPeer::connect_with_retry(&url, 50).await?;

    let peer_clone = peer.clone();
    tokio::spawn(async move {
        let _ = peer_clone.serve().await;
    });

    let resp = peer.call_raw(&method, Bytes::from(msg)).await?;
    println!("reply: {}", String::from_utf8_lossy(&resp));
    Ok(())
}

