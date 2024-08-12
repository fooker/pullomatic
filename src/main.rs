extern crate crossbeam;
extern crate crypto;
extern crate ctrlc;
extern crate git2;
extern crate hex;
extern crate json;
extern crate rouille;
#[macro_use]
extern crate serde_derive;
extern crate serde_humantime;
extern crate serde_yaml;
#[macro_use]
extern crate structopt;


use config::Config;
use repo::Repo;
use std::error::Error;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use std::sync::mpsc;
use structopt::StructOpt;


mod config;
mod repo;
mod ticker;
mod webhook;


#[derive(StructOpt, Debug)]
#[structopt(name = "pullomatic")]
struct Opts {
    #[structopt(short = "c",
                long = "config",
                default_value = "/etc/pullomatic")]
    config: String,

    #[structopt(short = "w",
                long = "webhook-listen",
                default_value = "localhost:8000")]
    webhook_listen: String,
}


pub static RUNNING: AtomicBool = AtomicBool::new(true);


fn main() {
    let opts = Opts::from_args();

    let config = Config::load(opts.config);
    let config = match config {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Failed to load config: {}", err);
            return;
        }
    };

    let repos: Arc<Vec<Arc<Repo>>> = Arc::new(config
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
        handles.push(webhook::serve(opts.webhook_listen.to_owned(), repos.clone(), producer.clone()));
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
            Ok(true) => {
                exec_hook(repo);
            }

            Ok(false) => {
                // Nothing changed
            }

            Err(err) => {
                eprintln!("[{}] Error while updating: {}", repo.name(), err.description());
            }
        }
    }

    for handle in handles {
        handle.join().unwrap();
    }
}


fn exec_hook(repo: Arc<Repo>) {
    if let Some(ref script) = repo.config().on_change {
        let mut child = Command::new("sh")
                .arg("-c")
                .arg(script)
                .current_dir(&repo.config().path)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to spawn script");

        let (stdout, stderr) = (child.stdout.take(), child.stderr.take());

        crossbeam::scope(|scope| {
            if let Some(stdout) = stdout {
                let stdout = BufReader::new(stdout);
                scope.spawn(|_| {
                    for line in stdout.lines() {
                        println!("[{}] {}", repo.name(), line.unwrap());
                    }
                });
            }

            if let Some(stderr) = stderr {
                let stderr = BufReader::new(stderr);
                scope.spawn(|_| {
                    for line in stderr.lines() {
                        eprintln!("[{}] {}", repo.name(), line.unwrap());
                    }
                });
            }
        });

        child.wait().expect("Failed to wait for script");
    }
}
