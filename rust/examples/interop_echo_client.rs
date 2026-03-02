use std::env;

use forpc::RpcPeer;

#[derive(prost::Message, Clone)]
struct EchoRequest {
    #[prost(string, tag = "1")]
    data: String,
}

#[derive(prost::Message, Clone)]
struct EchoResponse {
    #[prost(string, tag = "1")]
    result: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "tcp://127.0.0.1:24000".to_string());
    let msg = args.next().unwrap_or_else(|| "Hello".to_string());

    let peer = RpcPeer::connect_with_retry(&url, 50).await?;

    let peer_clone = peer.clone();
    tokio::spawn(async move {
        let _ = peer_clone.serve().await;
    });

    let resp: EchoResponse = peer
        .call(
            "Test/Echo",
            EchoRequest { data: msg },
        )
        .await?;

    println!("reply: {}", resp.result);
    Ok(())
}
