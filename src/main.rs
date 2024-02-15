use prover_api::{create_router, init_tracing};
use tokio::net::TcpListener;
use tracing::debug;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    init_tracing();

    let router = create_router().with_state(Default::default());

    let port = std::env::var("PORT").unwrap_or("3000".to_string());

    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    debug!("Listening on: {}", listener.local_addr()?);

    axum::serve(listener, router).await?;

    Ok(())
}
