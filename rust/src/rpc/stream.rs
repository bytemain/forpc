use std::sync::Arc;
use tokio::sync::mpsc;
use fory::Reader;
use fory_core::{Serializer, ForyDefault};
use super::peer::RpcPeer;
use super::protocol::{Packet, Status, frame_kind};
use super::error::{RpcError, RpcResult};
use super::status::StatusCode;

pub struct BidiStream<Req, Resp> {
    pub(crate) stream_id: u32,
    pub(crate) peer: Arc<RpcPeer>,
    pub(crate) recv_rx: mpsc::Receiver<Packet>,
    pub(crate) _marker: std::marker::PhantomData<(Req, Resp)>,
}

impl<Req: Serializer + Send + Sync + 'static, Resp: Serializer + ForyDefault + Send + Sync + 'static> BidiStream<Req, Resp> {
    pub async fn send(&self, message: Req) -> RpcResult<()> {
        let payload = self.peer.user_serialize(&message).await?;
        self.peer.send_stream_data(self.stream_id, payload).await
    }
    
    pub async fn recv(&mut self) -> RpcResult<Option<Resp>> {
        match self.recv_rx.recv().await {
            Some(packet) => {
                match packet.kind {
                    frame_kind::DATA => {
                        let msg: Resp = self.peer.user_deserialize(&packet.payload).await?;
                        Ok(Some(msg))
                    }
                    frame_kind::TRAILERS => {
                        let fory = self.peer.proto_fory.lock().await;
                        let mut reader = Reader::new(&packet.payload);
                        let status: Status = fory.deserialize_from(&mut reader)
                             .map_err(|e| RpcError::new(StatusCode::INTERNAL, e.to_string()))?;
                        
                        if status.is_ok() {
                            Ok(None)
                        } else {
                            Err(RpcError::from_status(status))
                        }
                    }
                    _ => Ok(None),
                }
            }
            None => Ok(None),
        }
    }
    
    pub async fn close_send(&self) -> RpcResult<()> {
        self.peer
            .send_stream_trailers(self.stream_id, &Status::ok())
            .await
    }
}
