use std::sync::Arc;
use tokio::sync::mpsc;
use fory::Reader;
use fory_core::{Serializer, ForyDefault};
use super::peer::RpcPeer;
use super::protocol::{Packet, Status, frame_kind};
use super::error::{RpcError, RpcResult};
use super::status::StatusCode;
use bytes::Bytes;

pub struct BidiStream<Req, Resp> {
    pub(crate) stream_id: u32,
    pub(crate) peer: Arc<RpcPeer>,
    pub(crate) recv_rx: mpsc::Receiver<Packet>,
    pub(crate) _marker: std::marker::PhantomData<(Req, Resp)>,
}

impl<Req: Serializer + Send + Sync + 'static, Resp: Serializer + ForyDefault + Send + Sync + 'static> BidiStream<Req, Resp> {
    pub async fn send(&self, message: Req) -> RpcResult<()> {
        let payload = {
            let fory = self.peer.user_fory.lock().await;
            Bytes::from(fory.serialize(&message).map_err(|e| RpcError::new(StatusCode::INTERNAL, e.to_string()))?)
        };
        
        self.peer.send_packet(Packet::data(self.stream_id, payload)).await
    }
    
    pub async fn recv(&mut self) -> RpcResult<Option<Resp>> {
        match self.recv_rx.recv().await {
            Some(packet) => {
                match packet.kind {
                    frame_kind::DATA => {
                        let mut reader = Reader::new(&packet.payload);
                        let fory = self.peer.user_fory.lock().await;
                        let msg: Resp = fory.deserialize_from(&mut reader)
                             .map_err(|e| RpcError::new(StatusCode::INTERNAL, e.to_string()))?;
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
        let status = Status::ok();
        let packet = {
            let mut fory = self.peer.proto_fory.lock().await;
            Packet::trailers(self.stream_id, &status, &mut fory).map_err(|e| RpcError::new(StatusCode::INTERNAL, e.to_string()))?
        };
        self.peer.send_packet(packet).await
    }
}
