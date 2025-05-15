use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use futures::future::FutureExt;
use repo::Repo;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{Instrument, Level, debug, error, info_span, trace};

mod config;
mod repo;
mod webhook;

#[derive(Parser, Debug)]
#[command(version, about, author, name = "pullomatic")]
struct Args {
    #[arg(short = 'c', long = "config", default_value = "/etc/pullomatic")]
    config: PathBuf,

    #[arg(short = 'w', long = "webhook-listen", default_value = "localhost:8000")]
    webhook_listen: String,

    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, default_value = "0")]
    verbose: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(match args.verbose {
            0 => Level::WARN,
            1 => Level::INFO,
            2 => Level::DEBUG,
            _ => Level::TRACE,
        })
        .init();

    let config = Config::load(&args.config)
        .await
        .with_context(|| format!("Failed to load config from {}", args.config.display()))?;

    let repos: Vec<Arc<Repo>> = config
        .into_iter()
        .map(|(name, config)| Arc::new(Repo::new(name, config)))
        .collect();

    // A single global worker queue to serialize all update checks
    let (producer, mut consumer) = tokio::sync::mpsc::channel(repos.len() + 1);

    let running = CancellationToken::new();
    let tasks = TaskTracker::new();

    // Create periodic update tasks for all repos
    for repo in repos.iter().cloned() {
        let Some(interval) = &repo.config.interval else {
            continue;
        };

        let interval = interval.interval;

        let producer = producer.clone();
        let running = running.clone();

        tasks.spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        producer.send(repo.clone()).await.expect("Receiver closed");
                    }

                    _ = running.cancelled() => {
                        break;
                    }
                }
            }
        });
    }

    // Start web server
    tasks.spawn(webhook::serve(
        args.webhook_listen,
        running.clone(),
        producer.clone(),
        &repos,
    ));

    // Listen for shutdown signal
    tasks.spawn({
        let running = running.clone();

        async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for Ctrl+C");
            debug!("Received Ctrl+C. Shutting down.");
            running.cancel();
        }
    });

    // Handle refresh tasks from queue
    loop {
        tokio::select! {
            _ = running.cancelled() => {
                debug!("Shutting down");
                break;
            }

            repo = consumer.recv() => {
                let Some(repo) = repo else {
                    break;
                };

                let task = precess(repo.clone());
                let task = task.map(|result| match result {
                    Ok(_) => { trace!("Update successful"); }
                    Err(err) => { error!("Error while updating: {:#}", err); }
                });
                let task = task.instrument(info_span!("Update repo", repo = repo.name));

                tasks.spawn(task);
            }
        }
    }

    tasks.close();
    tasks.wait().await;

    return Ok(());
}

async fn precess(repo: Arc<Repo>) -> Result<()> {
    let changed = repo
        .update()
        .await
        .with_context(|| format!("Error while update {}", repo.name))?;

    if !changed {
        trace!("No changes");
        return Ok(());
    }

    let Some(ref script) = repo.config.on_change else {
        trace!("No script to execute");
        return Ok(());
    };

    let mut child = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .current_dir(&repo.config.path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn script")?;

    let mut stdout = BufReader::new(child.stdout.take().expect("Failed to take stdout")).lines();
    let mut stderr = BufReader::new(child.stderr.take().expect("Failed to take stderr")).lines();

    loop {
        tokio::select! {
            Ok(Some(line)) = stdout.next_line() => {
                trace!("> {}", line);
            }

            Ok(Some(line)) = stderr.next_line() => {
                trace!("! {}", line);
            }

            else => break,
        }
    }

    child.wait().await.context("Failed to wait for script")?;

    return Ok(());
}
