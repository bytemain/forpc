use std::env;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use mini_rpc::{RpcListener, RpcPeer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let mut args = env::args().skip(1);
    let listen_url = args
        .next()
        .unwrap_or_else(|| "ipc:///tmp/mini_rpc_proxy.ipc".to_string());
    let upstream_url = args
        .next()
        .unwrap_or_else(|| "ipc:///tmp/mini_rpc_upstream.ipc".to_string());

    let listener = RpcListener::bind(&listen_url).await?;

    loop {
        let downstream = listener.accept().await?;
        tokio::spawn(proxy_connection(downstream, upstream_url.clone()));
    }
}

async fn proxy_connection(
    downstream: Arc<RpcPeer>,
    upstream_url: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let upstream = RpcPeer::connect_with_retry(&upstream_url, 10).await?;

    let downstream_in = downstream.transport_arc();
    let downstream_out = downstream.transport_arc();
    let upstream_in = upstream.transport_arc();
    let upstream_out = upstream.transport_arc();

    let a_to_b = tokio::spawn(async move {
        loop {
            let pkt: Bytes = downstream_in.recv().await?;
            upstream_in.send(pkt).await?;
        }
        #[allow(unreachable_code)]
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    let b_to_a = tokio::spawn(async move {
        loop {
            let pkt: Bytes = upstream_out.recv().await?;
            downstream_out.send(pkt).await?;
        }
        #[allow(unreachable_code)]
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    let _ = tokio::select! {
        r = a_to_b => r?,
        r = b_to_a => r?,
    };

    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}
