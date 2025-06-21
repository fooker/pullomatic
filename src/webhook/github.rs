use crate::config::GitHubWebhook;
use crate::repo::Repo;
use anyhow::Result;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::sync::Arc;
use tracing::{debug, trace};

pub(super) fn router(
    config: GitHubWebhook,
    producer: tokio::sync::mpsc::Sender<Arc<Repo>>,
    repo: Arc<Repo>,
) -> Router {
    Router::<_>::new()
        .route("/", post(handle))
        .with_state((config, producer, repo))
}

async fn handle(
    State((config, producer, repo)): State<(
        GitHubWebhook,
        tokio::sync::mpsc::Sender<Arc<Repo>>,
        Arc<Repo>,
    )>,
    headers: HeaderMap,
    body: String,
) -> Result<(), (StatusCode, &'static str)> {
    // Check if the signature matches the secret
    if let Some(ref secret) = config.secret {
        let signature = headers
            .get("X-Hub-Signature")
            .ok_or((StatusCode::UNAUTHORIZED, "Signature missing"))?
            .as_bytes();
        let signature = signature
            .strip_prefix(b"sha1=")
            .ok_or((StatusCode::UNAUTHORIZED, "Signature prefix missing"))?;
        let signature =
            hex::decode(signature).map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid signature"))?;

        let mut hmac = Hmac::<Sha1>::new_from_slice(
            secret
                .load()
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load secret"))?
                .as_bytes(),
        )
        .expect("HMAC can take key of any size");
        hmac.update(body.as_bytes());

        if let Err(_) = hmac.verify_slice(&signature) {
            return Err((StatusCode::UNAUTHORIZED, "Signature mismatch"));
        }
    }

    // Only allow 'push' or 'ping' events
    let event = headers
        .get("X-GitHub-Event")
        .ok_or((StatusCode::BAD_REQUEST, "Not a GitHub webhook request"))?;
    trace!("Got GitHub event: {:?}", event);

    if event == "ping" {
        return Ok(());
    } else if event != "push" {
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

    Ok(())
}
