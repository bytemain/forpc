use std::env;
use std::sync::Arc;

use bytes::Bytes;
use forpc::{RpcListener, RpcPeer, RpcResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "tcp://127.0.0.1:24002".to_string());

    let listener = RpcListener::bind(&url).await?;
    let peer = listener.accept().await?;

    peer.register_raw("Raw/Echo", |payload: Bytes, _meta, _peer: Arc<RpcPeer>| async move {
        Ok(payload) as RpcResult<Bytes>
    })
    .await;

    peer.serve().await?;
    Ok(())
}
