use std::env;
use std::sync::Arc;

use forpc::RpcPeer;
use forpc::pb::test::{TestRequest, TestResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let mut args = env::args().skip(1);
    let proxy_url = args
        .next()
        .unwrap_or_else(|| "ipc:///tmp/forpc_proxy.ipc".to_string());

    let peer = RpcPeer::connect_with_retry(&proxy_url, 10).await?;

    let peer = Arc::new(peer);
    let peer_task = peer.clone();
    tokio::spawn(async move {
        let _ = peer_task.serve().await;
    });

    let resp: TestResponse = peer
        .call(
            "Test/Echo",
            TestRequest {
                data: "Hello".to_string(),
            },
        )
        .await?;

    println!("{}", resp.result);
    Ok(())
}
