use crate::config::{Config, Credentials};
use anyhow::{Context, Result};
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, info, trace};

#[derive(Debug)]
struct RepoState {
    last_checked: Option<Instant>,
    last_changed: Option<Instant>,
}

#[derive(Debug)]
pub struct Repo {
    pub name: String,
    pub config: Config,

    state: Mutex<RepoState>,
}

const TARGET_REF: &str = "refs/pullomatic";

impl Repo {
    pub fn new(name: String, config: Config) -> Self {
        Self {
            name,
            config,

            state: Mutex::new(RepoState {
                last_checked: None,
                last_changed: None,
            }),
        }
    }

    pub async fn update(&self) -> Result<bool> {
        let mut state = self.state.lock().await;

        let now = Some(Instant::now());
        state.last_checked = now;

        let path = self.config.path.as_path();

        let repository: git2::Repository;
        if path.exists() {
            debug!("Using existing repository");
            repository = tokio::task::block_in_place(|| git2::Repository::open(path))?;
        } else {
            debug!("Initialized new repository");
            tokio::fs::create_dir_all(path).await?;
            repository = tokio::task::block_in_place(|| git2::Repository::init(path))?;
        }

        let mut remote = repository.remote_anonymous(&self.config.remote_url)?;

        let mut remote_cb = git2::RemoteCallbacks::new();
        match &self.config.credentials {
            None => {}

            Some(Credentials::Password(password)) => {
                let plain_username = password.username.clone();
                let plain_password = password.password.load().await?;

                remote_cb.credentials(move |url, username, allowed| {
                    trace!("cred: url = {:?}", url);
                    trace!("cred: username = {:?}", username);
                    trace!("cred: allowed = {:?}", allowed);

                    if username != Some(plain_username.as_str()) {
                        return Err(git2::Error::from_str("Invalid username"));
                    }

                    if allowed.contains(git2::CredentialType::USERNAME) {
                        return git2::Cred::username(&plain_username);
                    }

                    if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                        return git2::Cred::userpass_plaintext(&plain_username, &plain_password);
                    }

                    Err(git2::Error::from_str("Unsupported authentication"))
                });
            }

            Some(Credentials::SSH(ssh)) => {
                let ssh_username = ssh.username.as_str();

                let ssh_private_key = ssh.private_key.load().await?;
                let ssh_public_key = ssh.public_key.as_ref().map(String::as_ref);

                let ssh_passphrase = match ssh.passphrase {
                    Some(ref passphrase) => Some(passphrase.load().await?),
                    None => None,
                };

                remote_cb.credentials(move |url, username, allowed| {
                    trace!("cred: url = {:?}", url);
                    trace!("cred: username = {:?}", username);
                    trace!("cred: allowed = {:?}", allowed);

                    if allowed.contains(git2::CredentialType::USERNAME) {
                        return git2::Cred::username(ssh_username);
                    }

                    if allowed.contains(git2::CredentialType::SSH_KEY) {
                        return git2::Cred::ssh_key_from_memory(
                            ssh_username,
                            ssh_public_key,
                            ssh_private_key.as_ref(),
                            ssh_passphrase.as_ref().map(String::as_ref),
                        );
                    }

                    Err(git2::Error::from_str("Unsupported authentication"))
                });
            }
        }

        debug!("Fetching data from remote");
        tokio::task::block_in_place(|| {
            // Fetch the remote branch head ref into our target ref
            remote
                .fetch(
                    &[&format!("+{}:{}", self.config.remote_ref(), TARGET_REF)],
                    Some(
                        git2::FetchOptions::new()
                            .prune(git2::FetchPrune::On)
                            .download_tags(git2::AutotagOption::None)
                            .remote_callbacks(remote_cb),
                    ),
                    None,
                )
                .with_context(|| {
                    format!(
                        "Failed to fetch data from remote: {}",
                        self.config.remote_ref()
                    )
                })
        })?;
        debug!("Fetched data from remote");

        let latest_obj = repository.revparse_single("HEAD").ok();
        let target_obj = repository
            .revparse_single(TARGET_REF)
            .expect("target ref fetched");

        // If the remote ref is the same as the local HEAD ref, we're up to date
        if let Some(ref latest_obj) = latest_obj {
            if latest_obj.id() == target_obj.id() {
                debug!("Already up to date");
                return Ok(false);
            }
        }

        tokio::task::block_in_place(|| {
            // Reset the local HEAD ref to the remote ref, and force a checkout
            repository
                .reset(
                    &target_obj,
                    git2::ResetType::Hard,
                    Some(
                        git2::build::CheckoutBuilder::new()
                            .force()
                            .remove_untracked(true),
                    ),
                )
                .with_context(|| format!("Failed to reset repo to target ref: {}", target_obj.id()))
        })?;

        info!("Updated to {}", target_obj.id());
        state.last_changed = now;

        Ok(true)
    }

    #[allow(unused)]
    pub async fn last_checked(&self) -> Option<Instant> {
        let state = self.state.lock().await;
        state.last_checked
    }
}
