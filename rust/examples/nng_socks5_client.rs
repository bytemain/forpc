use std::collections::HashMap;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use fory::ForyObject;
use forpc::{BidiStream, RpcPeer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(ForyObject, Debug, Clone)]
struct TcpChunk {
    data: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let mut args = env::args().skip(1);
    let socks_listen = args
        .next()
        .unwrap_or_else(|| "127.0.0.1:1080".to_string());
    let server_url = args
        .next()
        .unwrap_or_else(|| "tcp://127.0.0.1:4000".to_string());

    let peer = RpcPeer::connect_with_retry(&server_url, 10).await?;
    peer.register_type_by_namespace::<TcpChunk>("forpc.tunnel", "TcpChunk")
        .await?;
    let peer = peer;

    let peer_task = peer.clone();
    tokio::spawn(async move {
        let _ = peer_task.serve().await;
    });

    let listener = tokio::net::TcpListener::bind(&socks_listen).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let peer = peer.clone();
        tokio::spawn(async move {
            let _ = handle_socks5(stream, peer).await;
        });
    }
}

async fn handle_socks5(
    mut stream: tokio::net::TcpStream,
    peer: Arc<RpcPeer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await?;
    if buf[0] != 0x05 {
        return Err("unsupported socks version".into());
    }
    let nmethods = buf[1] as usize;
    let mut methods = vec![0u8; nmethods];
    stream.read_exact(&mut methods).await?;
    stream.write_all(&[0x05, 0x00]).await?;

    let mut hdr = [0u8; 4];
    stream.read_exact(&mut hdr).await?;
    if hdr[0] != 0x05 {
        return Err("unsupported socks version".into());
    }
    if hdr[1] != 0x01 {
        stream.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
        return Err("only CONNECT supported".into());
    }
    let atyp = hdr[3];
    let host = match atyp {
        0x01 => {
            let mut ip = [0u8; 4];
            stream.read_exact(&mut ip).await?;
            IpAddr::V4(Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])).to_string()
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            let mut name = vec![0u8; len[0] as usize];
            stream.read_exact(&mut name).await?;
            String::from_utf8(name)?
        }
        0x04 => {
            let mut ip = [0u8; 16];
            stream.read_exact(&mut ip).await?;
            IpAddr::from(ip).to_string()
        }
        _ => {
            stream.write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
            return Err("unsupported ATYP".into());
        }
    };

    let mut port_buf = [0u8; 2];
    stream.read_exact(&mut port_buf).await?;
    let port = u16::from_be_bytes(port_buf);
    let dst = format!("{}:{}", host, port);

    let mut meta = HashMap::new();
    meta.insert("dst".to_string(), dst);
    let mut tunnel: BidiStream<TcpChunk, TcpChunk> = peer.stream_with_metadata("Tunnel/Tcp", meta).await?;

    let bnd = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
    let mut resp = vec![0x05, 0x00, 0x00, 0x01];
    resp.extend_from_slice(&match bnd.ip() {
        IpAddr::V4(v4) => v4.octets().to_vec(),
        IpAddr::V6(_) => vec![0, 0, 0, 0],
    });
    resp.extend_from_slice(&bnd.port().to_be_bytes());
    stream.write_all(&resp).await?;

    let (mut r, mut w) = stream.into_split();
    let mut buf = [0u8; 16 * 1024];
    let mut local_closed = false;
    loop {
        tokio::select! {
            n = r.read(&mut buf), if !local_closed => {
                let n = n?;
                if n == 0 {
                    local_closed = true;
                    let _ = tunnel.close_send().await;
                    continue;
                }
                tunnel.send(TcpChunk { data: buf[..n].to_vec() }).await?;
            }
            msg = tunnel.recv() => {
                match msg? {
                    Some(chunk) => {
                        if !chunk.data.is_empty() {
                            w.write_all(&chunk.data).await?;
                        }
                    }
                    None => break,
                }
            }
        }
    }
    Ok(())
}
