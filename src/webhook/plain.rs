use crate::config::PlainWebhook;
use crate::repo::Repo;
use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::Router;
use std::sync::Arc;
use tracing::debug;

pub(super) fn router(
    config: PlainWebhook,
    producer: tokio::sync::mpsc::Sender<Arc<Repo>>,
    repo: Arc<Repo>,
) -> Router {
    Router::new()
        .route("/", post(handle))
        .with_state((config, producer, repo))
}

async fn handle(
    State((_config, producer, repo)): State<(
        PlainWebhook,
        tokio::sync::mpsc::Sender<Arc<Repo>>,
        Arc<Repo>,
    )>,
) -> Result<(), (StatusCode, &'static str)> {
    debug!("Trigger update from hook");
    producer.send(repo.clone()).await.expect("Receiver dropped");

    Ok(())
}
