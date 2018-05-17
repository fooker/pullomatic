extern crate git2;
#[macro_use]
extern crate serde_derive;
extern crate toml;

use config::Config;
use repo::Repo;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Instant,Duration};

mod config;
mod repo;


fn handle(repo: &mut Repo) {
    match repo.update() {
        Ok(true) => {}
        Ok(false) => {}
        Err(err) => {}
    }
}

fn main() {
    let repos: Arc<Mutex<Vec<_>>> = Arc::new(Mutex::new(Config::load("/etc/pullomat").expect("Failed to load config")
                                                                                .into_iter()
                                                                                .map(|(name, config)| Repo::new(name, config))
                                                                                .collect()));

    // Start periodic update tasks
    let ticker = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));

            // Scan for repos needing update
            let mut repos = repos.lock().unwrap();
            for repo in repos.iter_mut() {
                if let Some(interval) = repo.config.interval {
                    if repo.last_updated().map_or(true, |t| t + interval < Instant::now()) {
                        handle(repo);
                    }
                }
            }
        }
    });

    ticker.join();
}
