extern crate ctrlc;
extern crate git2;
extern crate rouille;
#[macro_use]
extern crate serde_derive;
extern crate toml;


use config::Config;
use repo::Repo;
use std::process::Command;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use std::sync::mpsc;


mod config;
mod repo;
mod ticker;
mod webhook;


pub static RUNNING: AtomicBool = AtomicBool::new(true);


fn main() {
    let repos: Arc<Vec<Arc<Repo>>> = Arc::new(
        Config::load("/etc/pullomat")
                .expect("Failed to load config")
                .into_iter()
                .map(|(name, config)| Arc::new(Repo::new(name, config)))
                .collect());

    // Create worker queue
    let (producer, consumer) = mpsc::sync_channel(0);

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
        RUNNING.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Handle updates
    for repo in consumer {
        match repo.update() {
            Ok(Some(changes)) => {
                if let Some(ref script) = repo.config().on_change {
                    let status = Command::new("sh")
                            .arg("-c")
                            .arg(script)
                            .current_dir(&repo.config().path)
                            .status();

                    match status {
                        Ok(status) => {

                        }
                        Err(err) => {
                            eprintln!("[{}] Error while executing script: {:?}", repo.name(), err);
                        }
                    }
                }
            }
            Ok(None) => {
                // Nothing changed
            }
            Err(err) => {
                eprintln!("[{}] Error while updating: {:?}", repo.name(), err);
            }
        }
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
