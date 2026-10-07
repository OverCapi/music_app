use music_group_server::{serve, Config};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), music_group_server::Error> {
    let config = Config::from_env()?;
    let listener = TcpListener::bind(config.addr)
        .await
        .map_err(|e| format!("cannot listen on {}: {e}", config.addr))?;
    eprintln!(
        "music-group-server listening on http://{} (db: {})",
        listener.local_addr()?,
        config.db_path.display()
    );
    serve(listener, config).await
}
