use crate::config::Webhook;
use crate::repo::Repo;
use anyhow::Result;
use axum::routing::get;
use axum::Router;
use std::future::Future;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

mod github;
mod gitlab;
mod plain;

async fn root() -> &'static str {
    "pullomatic webhook server"
}

pub fn serve(
    addr: String,
    running: CancellationToken,
    producer: tokio::sync::mpsc::Sender<Arc<Repo>>,
    repos: &[Arc<Repo>],
) -> impl Future<Output = Result<()>> + use<> {
    let mut app = Router::new().route("/", get(root));

    for repo in repos {
        let Some(ref config) = repo.config.webhook else {
            continue;
        };

        app = app.nest(
            &format!("/{}", repo.name),
            match config {
                Webhook::Plain(config) => {
                    plain::router(config.clone(), producer.clone(), repo.clone())
                }
                Webhook::GitHub(config) => {
                    github::router(config.clone(), producer.clone(), repo.clone())
                }
                Webhook::GitLab(config) => {
                    gitlab::router(config.clone(), producer.clone(), repo.clone())
                }
            },
        );
    }

    async move {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(running.cancelled_owned())
            .await?;

        Ok(())
    }
}

// fn handle(repo: &Repo, request: &Request) -> Result<bool, String> {
//     if let Some(ref config) = repo.config().webhook {
//         return match config {
//             &Webhook::Plain(ref config) => plain::handle(&repo, config, request),
//             &Webhook::GitHub(ref config) => github::handle(&repo, config, request),
//             &Webhook::GitLab(ref config) => gitlab::handle(&repo, config, request),
//         };
//     } else {
//         return Err("Repository not configured for webhooks".to_owned());
//     }
// }
//
// pub fn serve(
//     addr: String,
//     repos: Arc<Vec<Arc<Repo>>>,
//     producer: SyncSender<Arc<Repo>>,
// ) -> JoinHandle<()> {
//     return thread::spawn(move || {
//         let server = Server::new(addr, move |request: &Request| {
//             let _request = info_span!("Handle webhook request").entered();
//
//             // Get the path without the leading slash
//             let path = &request.url()[1..];
//
//             // Try find the repo this the path interpreted as name
//             let repo = repos.iter().find(move |repo| repo.name() == path).cloned();
//             let Some(repo) = repo else {
//                 return Response::empty_404();
//             };
//
//             let _repo = info_span!("Handle webhook request", repo = repo.name()).entered();
//
//             match handle(&repo, request) {
//                 Ok(trigger) => {
//                     if trigger {
//                         producer.send(repo).unwrap();
//                     }
//
//                     return Response::empty_204();
//                 }
//
//                 Err(error) => {
//                     return Response::text(error).with_status_code(400);
//                 }
//             }
//         })
//         .expect("Failed to start server");
//
//         use super::RUNNING;
//         while RUNNING.load(Ordering::SeqCst) {
//             server.poll();
//         }
//     });
// }
