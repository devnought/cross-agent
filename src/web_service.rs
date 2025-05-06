use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use axum::{Router, body::Body, extract::State, response::IntoResponse, routing::get};
use futures::TryStreamExt;

use crate::stream::build_stream;

#[derive(Debug, Clone)]
struct ServiceState {
    roots: Vec<PathBuf>,
}

pub async fn start<IR, R>(port: u16, root: IR) -> anyhow::Result<()>
where
    IR: IntoIterator<Item = R>,
    R: AsRef<Path>,
{
    let state = ServiceState {
        roots: root.into_iter().map(|p| p.as_ref().to_path_buf()).collect(),
    };

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/fs", get(download_filesystem))
        .with_state(state);

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    let listener = tokio::net::TcpListener::bind(address).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn download_filesystem(State(state): State<ServiceState>) -> impl IntoResponse {
    let stream = build_stream(state.roots.into_iter());
    Body::from_stream(stream)
}
