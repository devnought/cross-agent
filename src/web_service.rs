use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::{Router, body::Body, response::IntoResponse, routing::get};

use crate::files::build_stream;

pub async fn start(port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/fs", get(download_filesystem));

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    let listener = tokio::net::TcpListener::bind(address).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn download_filesystem() -> impl IntoResponse {
    let stream = build_stream("/screenshots");
    Body::from_stream(stream)
}
