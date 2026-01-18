use std::env;
use std::sync::Arc;

use fory::ForyObject;
use mini_rpc::RpcPeer;

#[path = "http_proxy/common.rs"]
mod http_proxy_common;
use http_proxy_common::HttpProxyTransport;

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
    let base_url = args
        .next()
        .unwrap_or_else(|| "http://127.0.0.1:3000".to_string());

    let session_id = HttpProxyTransport::connect(&base_url).await?;
    let transport = HttpProxyTransport::new(&base_url, &session_id);

    let peer = Arc::new(RpcPeer::new(transport, true));
    peer.register_type::<TestRequest>(4).await?;
    peer.register_type::<TestResponse>(5).await?;

    let peer_clone = peer.clone();
    tokio::spawn(async move {
        let _ = peer_clone.serve().await;
    });

    let resp: TestResponse = peer
        .call(
            "Test/Echo",
            TestRequest {
                data: "Hello".to_string(),
            },
        )
        .await?;

    if resp.result != "Hello" {
        return Err(format!("unexpected response: {}", resp.result).into());
    }

    println!("ok");
    Ok(())
}
