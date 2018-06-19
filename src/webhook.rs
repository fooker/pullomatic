use config::Config;
use repo::Repo;
use rouille::{Request, Response, Server, url::Url};
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering, Mutex};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::borrow::Cow;


pub fn serve(repos: Arc<Vec<Arc<Repo>>>,
             producer: SyncSender<Arc<Repo>>) -> JoinHandle<()> {
    return thread::spawn(move || {
        let server = Server::new("localhost:8000", move |request: &Request| {
            if request.method() != "POST" {
                return Response::empty_400();
            }

            // Get the path without the leading slash
            let path = &request.url()[1..];

            // Try find the repo this the path interpreted as name
            let repo = repos.iter().find(move |repo| repo.name() == path).cloned();
            if let Some(repo) = repo {
                producer.send(repo).unwrap();
                return Response::empty_204();
            } else {
                return Response::empty_404();
            }
        }).expect("Failed to start server");

        use super::RUNNING;
        while RUNNING.load(Ordering::SeqCst) {
            server.poll();
        }
    });
}
