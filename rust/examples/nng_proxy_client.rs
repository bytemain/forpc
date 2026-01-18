use std::env;
use std::sync::Arc;

use fory::ForyObject;
use mini_rpc::RpcPeer;

#[derive(ForyObject, Debug, Clone, PartialEq)]
struct TestRequest {
    data: String,
}

#[derive(ForyObject, Debug, Clone, PartialEq)]
struct TestResponse {
    result: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let mut args = env::args().skip(1);
    let proxy_url = args
        .next()
        .unwrap_or_else(|| "ipc:///tmp/mini_rpc_proxy.ipc".to_string());

    let peer = RpcPeer::connect_with_retry(&proxy_url, 10).await?;
    peer.register_type::<TestRequest>(4).await?;
    peer.register_type::<TestResponse>(5).await?;

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

