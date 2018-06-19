use config::Config;
use repo::Repo;
use rouille::{Request, Response, Server, url::Url};
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};


pub fn serve(repos: Arc<Vec<Arc<Repo>>>,
             producer: Sender<Arc<Repo>>) -> JoinHandle<()> {
    let server = Server::new("localhost:8000", move |request: &Request| {
        if request.method() != "POST" {
            return Response::empty_404();
        }

        // Get the path without the leading slash
        let path = &request.url()[1..];

        // Try find the repo this the path interpreted as name
        let repo = repos.iter().find(move |repo| repo.name() == path);

        return Response::empty_404();
    }).expect("Failed to start server");

    return thread::spawn(move || {
        use super::running;
        while running.load(Ordering::SeqCst) {
            server.poll();
        }
    });
}
