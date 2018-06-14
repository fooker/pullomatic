extern crate git2;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate ctrlc;


use config::Config;
use repo::Repo;
use std::collections::HashMap;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering, Mutex};
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};


mod config;
mod repo;


const running: AtomicBool = AtomicBool::new(true);

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
    let tickers: Vec<JoinHandle<()>> = repos.iter().cloned().filter_map(|repo| {
        let interval = repo.lock().unwrap().config.interval;

        if let Some(interval) = interval {
            let producer = producer.clone();

            return Some(thread::spawn(move || {
                while running.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_secs(1));

                    if repo.lock().unwrap().last_updated().map_or(true, |t| t + interval < Instant::now()) {
                        producer.send(repo.clone());
                    }
                }

                println!("Done");
            }));
        } else {
            return None;
        }
    }).collect();

    // Handle Signals
    // FIXME: Does not work...
//    ctrlc::set_handler(move || {
//        running.store(false, Ordering::SeqCst);
//    }).expect("Error setting Ctrl-C handler");

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

    for ticker in tickers {
        ticker.join();
    }
}
