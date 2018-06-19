extern crate ctrlc;
extern crate git2;
#[macro_use]
extern crate rouille;
#[macro_use]
extern crate serde_derive;
extern crate toml;


use config::Config;
use repo::Repo;
use std::collections::HashMap;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};


mod config;
mod repo;
mod ticker;
mod webhook;


pub static running: AtomicBool = AtomicBool::new(true);


fn main() {
    let repos: Arc<Vec<Arc<Repo>>> = Arc::new(
        Config::load("/etc/pullomat")
                .expect("Failed to load config")
                .into_iter()
                .map(|(name, config)| Arc::new(Repo::new(name, config)))
                .collect());

    // Create worker queue
    let (producer, consumer) = mpsc::channel();

    // Handles for background tasks
    let mut handles = vec![];

    // Start periodic update tasks
    handles.extend(repos.iter()
                        .cloned()
                        .filter_map(|repo| ticker::ticker(repo, producer.clone())));

    // Start web server
    if repos.iter().any(|repo| repo.config().webhook.is_some()) {
        handles.push(webhook::serve(repos.clone(), producer.clone()));
    }

    // Ensure the initial producer is dropped, so the worker will stop if all other producers have died
    drop(producer);

    // Handle Signals
    ctrlc::set_handler(move || {
        println!("Terminating...");
        running.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Handle updates
    for repo in consumer {
        match repo.update() {
            Ok(true) => {}
            Ok(false) => {}
            Err(err) => {
                eprintln!("[{}] Error while updating: {:?}", repo.name(), err);
            }
        }
    }

    println!("Done");

    for handle in handles {
        handle.join();
    }
}
