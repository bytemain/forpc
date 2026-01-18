#![deny(clippy::all)]

use std::sync::Arc;

use bytes::Bytes;
use mini_rpc::{Request, Response, RpcListener, RpcPeer, StatusCode};
use napi::bindgen_prelude::{spawn, Buffer};
use napi::threadsafe_function::ThreadsafeFunction;
use napi::bindgen_prelude::Function;
use napi_derive::napi;

type RawHandlerTsfn = ThreadsafeFunction<Buffer, Buffer, Buffer, napi::Status, false>;

#[napi]
pub fn plus_100(input: u32) -> u32 {
  input + 100
}

#[napi]
pub struct Peer {
  inner: Arc<RpcPeer>,
}

#[napi]
impl Peer {
  #[napi(factory)]
  pub async fn connect(url: String) -> napi::Result<Self> {
    let peer = RpcPeer::connect_with_retry(&url, 50)
      .await
      .map_err(|e| napi::Error::new(napi::Status::GenericFailure, format!("rpc {}: {}", e.code, e.message)))?;

    let peer_clone = peer.clone();
    spawn(async move {
      let _ = peer_clone.serve().await;
    });

    Ok(Self { inner: peer })
  }

  #[napi]
  pub async fn call_raw(&self, method: String, payload: Buffer) -> napi::Result<Buffer> {
    let resp = self
      .inner
      .call_raw(&method, Bytes::from(payload.to_vec()))
      .await
      .map_err(|e| napi::Error::new(napi::Status::GenericFailure, format!("rpc {}: {}", e.code, e.message)))?;
    Ok(Buffer::from(resp.to_vec()))
  }
}

#[napi]
pub struct RawServer {
  _handler: Arc<RawHandlerTsfn>,
}

#[napi]
impl RawServer {
  #[napi(factory)]
  pub fn listen(url: String, method: String, handler: Function<Buffer, Buffer>) -> napi::Result<Self> {
    let tsfn = handler
      .build_threadsafe_function::<Buffer>()
      .build_callback(|ctx| Ok(ctx.value))?;
    let tsfn = Arc::new(tsfn);

    let url_clone = url.clone();
    let method_clone = method.clone();
    let tsfn_for_task = tsfn.clone();

    spawn(async move {
      let listener = match RpcListener::bind(&url_clone).await {
        Ok(v) => v,
        Err(_) => return,
      };
      let peer = match listener.accept().await {
        Ok(v) => v,
        Err(_) => return,
      };

      let tsfn_for_req = tsfn_for_task.clone();
      peer
        .register(&method_clone, move |req: Request, _peer: Arc<RpcPeer>| {
          let tsfn = tsfn_for_req.clone();
          async move {
            let mut payload = Bytes::new();
            if let Some(mut rx) = req.stream {
              while let Some(pkt) = rx.recv().await {
                if pkt.kind == 1 {
                  payload = pkt.payload;
                } else if pkt.kind == 2 {
                  break;
                }
              }
            }
            if payload.is_empty() {
              return Response::error_with_code(StatusCode::INVALID_ARGUMENT, "Missing payload");
            }

            match tsfn.call_async(Buffer::from(payload.to_vec())).await {
              Ok(buf) => Response::ok(Bytes::from(buf.to_vec())),
              Err(e) => Response::error_with_code(StatusCode::INTERNAL, e.to_string()),
            }
          }
        })
        .await;

      let _ = peer.serve().await;
    });

    Ok(Self { _handler: tsfn })
  }
}
