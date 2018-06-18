use config::Config;
use repo::Repo;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};

use hyper::{Body, Response, Server};
use hyper::service::service_fn_ok;
use hyper::rt::{self, Future};



pub fn serve(repos: Arc<Vec<Arc<Mutex<Repo>>>>,
             producer: Sender<Arc<Mutex<Repo>>>) {

    let addr = ([127, 0, 0, 1], 3000).into();

    let new_service = || {
        // This is the `Service` that will handle the connection.
        // `service_fn_ok` is a helper to convert a function that
        // returns a Response into a `Service`.
        service_fn_ok(|_| {
            Response::new(Body::from("asdasd"))
        })
    };

    let server = Server::bind(&addr)
            .serve(new_service)
            .map_err(|e| eprintln!("server error: {}", e));

    rt::run(server);
}
