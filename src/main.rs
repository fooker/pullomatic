extern crate git2;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate ctrlc;
extern crate hyper;


use config::Config;
use repo::Repo;
use std::collections::HashMap;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering, Mutex};
use std::sync::mpsc::{self,Sender,Receiver};
use std::thread::{self,JoinHandle};
use std::time::{Duration, Instant};


mod config;
mod webhook;
mod repo;


static running: AtomicBool = AtomicBool::new(true);


fn ticker(repo: Arc<Mutex<Repo>>,
          producer: Sender<Arc<Mutex<Repo>>>) -> Option<JoinHandle<()>> {
    let interval = repo.lock().unwrap().config.interval;

    if let Some(interval) = interval {
        let producer = producer.clone();

        return Some(thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                // TODO: Calculate sleep time instead of checking regulary

                // Check if update is outstanding and send it as task to the worker
                if repo.lock().unwrap().last_updated().map_or(true, |t| t + interval < Instant::now()) {
                    producer.send(repo.clone());
                }

                // Delay
                thread::sleep(Duration::from_secs(1));
            }
        }));
    } else {
        return None;
    }
}





fn main() {
    let repos: Arc<Vec<Arc<Mutex<Repo>>>> = Arc::new(
        Config::load("/etc/pullomat")
                .expect("Failed to load config")
                .into_iter()
                .map(|(name, config)| Arc::new(Mutex::new(Repo::new(name, config))))
                .collect());

    // Create worker queue
    let (producer, consumer) = mpsc::channel();

    // Start periodic update tasks
    let tickers: Vec<JoinHandle<()>> = repos.iter()
            .cloned()
            .filter_map(|repo| ticker(repo, producer.clone()))
            .collect();

    // Start web server
    if repos.iter().any(|repo| repo.lock().unwrap().config.webhook.is_some()) {
        webhook::serve(repos.clone(), producer.clone());
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
        let mut repo = repo.lock().unwrap();
        match repo.update() {
            Ok(true) => {}
            Ok(false) => {}
            Err(err) => {
                eprintln!("[{}] Error while updating: {:?}", repo.name, err);
            }
        }
    }

    println!("Done");

    for ticker in tickers {
        ticker.join();
    }
}
