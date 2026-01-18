use bytes::Bytes;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use mini_rpc::transport::nng::Transport;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectResponse {
    pub session_id: String,
}

#[derive(Clone)]
pub struct HttpProxyTransport {
    base_url: String,
    session_id: String,
    http: reqwest::Client,
}

impl HttpProxyTransport {
    pub fn new(base_url: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            session_id: session_id.into(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn connect(base_url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/connect", base_url.trim_end_matches('/'));
        let resp = reqwest::Client::new().post(url).send().await?;
        if !resp.status().is_success() {
            return Err(format!("connect failed: {}", resp.status()).into());
        }
        let body: ConnectResponse = resp.json().await?;
        Ok(body.session_id)
    }

    fn send_url(&self) -> String {
        format!("{}/session/{}/send", self.base_url, self.session_id)
    }

    fn recv_url(&self) -> String {
        format!("{}/session/{}/recv", self.base_url, self.session_id)
    }
}

#[async_trait::async_trait]
impl Transport for HttpProxyTransport {
    async fn send(&self, data: Bytes) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .http
            .post(self.send_url())
            .header("content-type", "application/octet-stream")
            .body(data)
            .send()
            .await?;
        if resp.status().is_success() {
            return Ok(());
        }
        Err(format!("http send failed: {}", resp.status()).into())
    }

    async fn recv(&self) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
        loop {
            let resp = self.http.get(self.recv_url()).send().await?;
            match resp.status() {
                StatusCode::OK => {
                    let b = resp.bytes().await?;
                    return Ok(Bytes::copy_from_slice(&b));
                }
                StatusCode::NO_CONTENT => continue,
                StatusCode::NOT_FOUND => return Err("session not found".into()),
                StatusCode::GONE => return Err("session closed".into()),
                code => return Err(format!("http recv failed: {}", code).into()),
            }
        }
    }
}

