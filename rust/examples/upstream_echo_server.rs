use std::env;

use forpc::RpcListener;

#[derive(prost::Message, Clone, PartialEq)]
struct TestRequest {
    #[prost(string, tag = "1")]
    data: String,
}

#[derive(prost::Message, Clone, PartialEq)]
struct TestResponse {
    #[prost(string, tag = "1")]
    result: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let mut args = env::args().skip(1);
    let url = args
        .next()
        .unwrap_or_else(|| "ipc:///tmp/forpc_upstream.ipc".to_string());

    let listener = RpcListener::bind(&url).await?;
    loop {
        let peer = listener.accept().await?;

        peer.register_type::<TestRequest>(4).await?;
        peer.register_type::<TestResponse>(5).await?;

        peer.register_unary("Test/Echo", |req: TestRequest, _meta, _peer| async move {
            Ok(TestResponse { result: req.data })
        })
        .await;

        tokio::spawn(async move {
            let _ = peer.serve().await;
        });
    }
}
