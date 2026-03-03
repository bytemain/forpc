use std::env;

use forpc::RpcListener;
use forpc::pb::test::{TestRequest, TestResponse};

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

        peer.register_unary("Test/Echo", |req: TestRequest, _meta, _peer| async move {
            Ok(TestResponse { result: req.data })
        })
        .await;

        tokio::spawn(async move {
            let _ = peer.serve().await;
        });
    }
}
