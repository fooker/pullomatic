use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use repo::Repo;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};

mod config;
mod repo;
mod ticker;
mod webhook;

#[derive(Parser, Debug)]
#[command(version, about, author, name = "pullomatic")]
struct Args {
    #[arg(short = 'c', long = "config", default_value = "/etc/pullomatic")]
    config: PathBuf,

    #[arg(short = 'w', long = "webhook-listen", default_value = "localhost:8000")]
    webhook_listen: String,
}

pub static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() -> Result<()> {
    let args = Args::parse();

    let config = Config::load(&args.config)
        .with_context(|| format!("Failed to load config from {}", args.config.display()))?;

    let repos: Arc<Vec<Arc<Repo>>> = Arc::new(
        config
            .into_iter()
            .map(|(name, config)| Arc::new(Repo::new(name, config)))
            .collect(),
    );

    // Create worker queue
    let (producer, consumer) = mpsc::sync_channel(0);

    // Handles for background tasks
    let mut handles = vec![];

    // Start periodic update tasks
    handles.extend(
        repos
            .iter()
            .cloned()
            .filter_map(|repo| ticker::ticker(repo, producer.clone())),
    );

    // Start web server
    if repos.iter().any(|repo| repo.config().webhook.is_some()) {
        handles.push(webhook::serve(
            args.webhook_listen.to_owned(),
            repos.clone(),
            producer.clone(),
        ));
    }

    // Ensure the initial producer is dropped, so the worker will stop if all other producers have died
    drop(producer);

    // Handle Signals
    ctrlc::set_handler(move || {
        println!("Terminating...");
        RUNNING.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Handle updates
    for repo in consumer {
        if let Err(err) = precess(repo.clone()) {
            eprintln!("[{}] Error while updating: {}", repo.name(), err);
        }
    }

    for handle in handles {
        handle.join().unwrap();
    }

    return Ok(());
}

fn precess(repo: Arc<Repo>) -> Result<()> {
    let changed = repo
        .update()
        .with_context(|| format!("Error while update {}", repo.name()))?;

    if !changed {
        return Ok(());
    }

    let Some(ref script) = repo.config().on_change else {
        return Ok(());
    };

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(script)
        .current_dir(&repo.config().path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn script")?;

    let (stdout, stderr) = (child.stdout.take(), child.stderr.take());

    let c = crossbeam::scope(|scope| {
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

    if let Err(err) = c {
        if let Some(string_err) = err.downcast_ref::<&str>() {
            panic!("Failed to execute crossbeam: {}", string_err);
        } else if let Some(string_err) = err.downcast_ref::<String>() {
            panic!("Failed to execute crossbeam: {}", string_err);
        } else {
            panic!("Failed to execute crossbeam: unknown error");
        }
    }

    child.wait().context("Failed to wait for script")?;

    return Ok(());
}
