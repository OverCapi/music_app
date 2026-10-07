use music_group_server::{Config, Error, serve};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // `RUST_LOG` overrides, e.g. `RUST_LOG=debug` to log every request.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    if let Err(e) = run().await {
        tracing::error!("{e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Error> {
    let config = Config::from_env()?;
    let listener = TcpListener::bind(config.addr)
        .await
        .map_err(|source| Error::Listen {
            addr: config.addr,
            source,
        })?;
    tracing::info!(
        "music-group-server listening on http://{} (db: {})",
        listener.local_addr()?,
        config.db_path.display()
    );
    serve(listener, config).await
}
