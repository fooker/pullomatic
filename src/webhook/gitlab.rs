use crate::config::GitLabWebhook;
use crate::repo::Repo;
use anyhow::Result;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use json;
use std::sync::Arc;
use tracing::{debug, trace};

pub(super) fn router(
    config: GitLabWebhook,
    producer: tokio::sync::mpsc::Sender<Arc<Repo>>,
    repo: Arc<Repo>,
) -> Router {
    return Router::new()
        .route("/", post(handle))
        .with_state((config, producer, repo));
}

async fn handle(
    State((config, producer, repo)): State<(
        GitLabWebhook,
        tokio::sync::mpsc::Sender<Arc<Repo>>,
        Arc<Repo>,
    )>,
    headers: HeaderMap,
    body: String,
) -> Result<(), (StatusCode, &'static str)> {
    // Check if the token matches
    if let Some(ref token) = config.token {
        if token
            != headers
                .get("X-Gitlab-Token")
                .ok_or((StatusCode::UNAUTHORIZED, "Token missing"))?
        {
            return Err((StatusCode::UNAUTHORIZED, "Token mismatch"));
        }
    }

    // Only allow 'push' or 'ping' events
    let event = headers
        .get("X-Gitlab-Event")
        .ok_or((StatusCode::BAD_REQUEST, "Not a GitLab webhook request"))?;
    trace!("Got GitLab event: {:?}", event);
    if event != "Push Hook" && event != "Push Event" {
        return Err((StatusCode::BAD_REQUEST, "Event not supported"));
    }

    // Parse the payload
    let payload = json::parse(&body).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid payload"))?;

    // Check if push is for our remote branch
    trace!("Got push event for '{}'", payload["ref"]);
    if config.check_branch.unwrap_or(true)
        && payload["ref"].as_str() != Some(&repo.config.remote_ref())
    {
        return Ok(());
    }

    debug!("Trigger update from hook");
    producer.send(repo.clone()).await.expect("Receiver dropped");

    return Ok(());
}
