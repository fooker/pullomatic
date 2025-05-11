use crate::repo::Repo;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, atomic::Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub fn ticker(repo: Arc<Repo>, producer: SyncSender<Arc<Repo>>) -> Option<JoinHandle<()>> {
    let Some(interval) = repo.config().interval.clone() else {
        return None;
    };

    let producer = producer.clone();

    return Some(thread::spawn(move || {
        use super::RUNNING;
        while RUNNING.load(Ordering::SeqCst) {
            // Check if update is outstanding and send it as task to the worker
            if repo
                .last_checked()
                .map_or(true, |t| t + interval.interval < Instant::now())
            {
                producer.send(repo.clone()).unwrap();
            }

            // Delay
            thread::sleep(Duration::from_secs(1));
        }
    }));
}
