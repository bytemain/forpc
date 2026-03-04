pub mod transport;
pub mod rpc;
pub mod pb;

pub use rpc::peer::{RpcPeer, Request, Response};
pub use rpc::listener::RpcListener;
pub use rpc::protocol::{Call, Status};
pub use rpc::error::{RpcError, RpcResult};
pub use rpc::status::StatusCode;
pub use rpc::stream::BidiStream;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    use crate::pb::test::{TestRequest, TestResponse};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_bidi_call() -> Result<(), BoxError> {
        let socket_path = format!("/tmp/test_forpc_{}.ipc", std::process::id());
        let url = format!("ipc://{}", socket_path);
        let url_server = url.clone();
        
        // Server
        let _server_handle = tokio::spawn(async move {
            let listener = RpcListener::bind(&url_server).await.unwrap();
            let peer = listener.accept().await.unwrap();
            
            // Register unary handler
            peer.register_unary("Test/Echo", |req: TestRequest, _meta, _peer| async move {
                Ok(TestResponse { result: req.data })
            }).await;
            
            // Serve
            peer.serve().await.unwrap();
        });
        
        sleep(Duration::from_millis(100)).await;
        
        // Client
        let peer = RpcPeer::connect(&url).await?;
        
        let peer_clone = peer.clone();
        tokio::spawn(async move {
            peer_clone.serve().await.unwrap();
        });
        
        // Call
        let resp: TestResponse = peer.call("Test/Echo", TestRequest { data: "Hello".to_string() }).await?;
        assert_eq!(resp.result, "Hello");
        
        let _ = std::fs::remove_file(socket_path);
        Ok(())
    }
}
