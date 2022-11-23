use crate::{
    config::Config,
    sync::{oneshot_when, shutdown_msg},
};
use ansi_term::Style;
use anyhow_ext::{bail, Result};
use axum::{http::StatusCode, response::IntoResponse, routing::get_service, Router};
use std::{io, net::SocketAddr};
use tokio::{net::TcpStream, task::JoinHandle};
use tower_http::services::ServeDir;

pub async fn spawn(config: &Config) -> Result<JoinHandle<()>> {
    let serve_dir = get_service(ServeDir::new("target/site")).handle_error(handle_error);

    let route = Router::new().nest("/", serve_dir.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], config.leptos.csr_port));

    if let Ok(_) = TcpStream::connect(&addr).await {
        let bold = Style::new().bold();
        bail!(
            "Server port {} already in use. You can set which port to use with {} in {} section {}",
            config.leptos.csr_port,
            bold.paint("csr_port"),
            bold.paint("Cargo.toml"),
            bold.paint("[package.metadata.leptos]"),
        );
    }

    let shutdown_rx = oneshot_when(shutdown_msg, "Server");

    log::info!("Serving client on {addr}");

    Ok(tokio::spawn(async move {
        match axum::Server::bind(&addr)
            .serve(route.into_make_service())
            .with_graceful_shutdown(async { drop(shutdown_rx.await.ok()) })
            .await
        {
            Ok(_) => log::debug!("Server stopped"),
            Err(e) => log::error!("Server {e}"),
        }
    }))
}

async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}
