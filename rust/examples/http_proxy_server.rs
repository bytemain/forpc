use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes as AxumBytes;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

use mini_rpc::transport::nng::{ClientTransport, Transport};

#[derive(Clone)]
struct AppState {
    upstream_url: String,
    sessions: Arc<RwLock<HashMap<String, Arc<Session>>>>,
}

struct Session {
    inbound_tx: mpsc::Sender<Bytes>,
    outbound_rx: Mutex<mpsc::Receiver<Bytes>>,
}

#[derive(serde::Serialize)]
struct ConnectResponse {
    session_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let mut args = env::args().skip(1);
    let listen = args
        .next()
        .unwrap_or_else(|| "127.0.0.1:3000".to_string());
    let upstream_url = args
        .next()
        .unwrap_or_else(|| "ipc:///tmp/mini_rpc_upstream.ipc".to_string());

    let state = AppState {
        upstream_url,
        sessions: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/connect", post(connect))
        .route("/session/:id/send", post(send_packet))
        .route("/session/:id/recv", get(recv_packet))
        .with_state(state)
        .layer(DefaultBodyLimit::disable());

    let addr: SocketAddr = listen.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn connect(State(state): State<AppState>) -> impl IntoResponse {
    let session_id = Uuid::new_v4().to_string();
    let session_id_for_task = session_id.clone();

    let (inbound_tx, mut inbound_rx) = mpsc::channel::<Bytes>(256);
    let (outbound_tx, outbound_rx) = mpsc::channel::<Bytes>(256);

    let session = Arc::new(Session {
        inbound_tx,
        outbound_rx: Mutex::new(outbound_rx),
    });

    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(session_id.clone(), session.clone());
    }

    let sessions = state.sessions.clone();
    let upstream_url = state.upstream_url.clone();

    tokio::spawn(async move {
        let upstream = match ClientTransport::new_with_retry(
            &upstream_url,
            10,
            Duration::from_millis(100),
        )
        .await
        {
            Ok(t) => Arc::new(t),
            Err(_) => {
                let mut sessions = sessions.write().await;
                sessions.remove(&session_id_for_task);
                return;
            }
        };

        let upstream_in = upstream.clone();
        let upstream_out = upstream.clone();

        let outbound_tx_in = outbound_tx.clone();
        let session_id_in = session_id_for_task.clone();
        let sessions_in = sessions.clone();

        let t_in = tokio::spawn(async move {
            while let Some(pkt) = inbound_rx.recv().await {
                if upstream_in.send(pkt).await.is_err() {
                    break;
                }
            }
        });

        let t_out = tokio::spawn(async move {
            loop {
                let pkt = match upstream_out.recv().await {
                    Ok(b) => b,
                    Err(_) => break,
                };
                if outbound_tx_in.send(pkt).await.is_err() {
                    break;
                }
            }
        });

        let _ = tokio::select! {
            _ = t_in => (),
            _ = t_out => (),
        };

        let mut sessions = sessions_in.write().await;
        sessions.remove(&session_id_in);
    });

    (StatusCode::OK, Json(ConnectResponse { session_id }))
}

async fn send_packet(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: AxumBytes,
) -> impl IntoResponse {
    let session = {
        let sessions = state.sessions.read().await;
        sessions.get(&id).cloned()
    };
    let Some(session) = session else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if session
        .inbound_tx
        .send(Bytes::copy_from_slice(&body))
        .await
        .is_err()
    {
        return StatusCode::GONE.into_response();
    }

    StatusCode::NO_CONTENT.into_response()
}

async fn recv_packet(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let session = {
        let sessions = state.sessions.read().await;
        sessions.get(&id).cloned()
    };
    let Some(session) = session else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mut rx = session.outbound_rx.lock().await;
    match tokio::time::timeout(Duration::from_secs(25), rx.recv()).await {
        Ok(Some(pkt)) => (StatusCode::OK, pkt).into_response(),
        Ok(None) => StatusCode::GONE.into_response(),
        Err(_) => StatusCode::NO_CONTENT.into_response(),
    }
}
