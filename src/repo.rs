use crate::config::{Config, Credentials};
use anyhow::{Context, Result};
use git2;
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

const TARGET_REF: &'static str = "refs/pullomatic";

impl Repo {
    pub fn new(name: String, config: Config) -> Self {
        return Self {
            name,
            config,

            state: Mutex::new(RepoState {
                last_checked: None,
                last_changed: None,
            }),
        };
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
        remote_cb.credentials(|url, username, allowed| {
            trace!("cred: url = {:?}", url);
            trace!("cred: username = {:?}", username);
            trace!("cred: allowed = {:?}", allowed);

            if allowed.contains(git2::CredentialType::USERNAME) {
                match self.config.credentials {
                    Some(Credentials::SSH(ref ssh)) => {
                        if let Some(ref username) = ssh.username {
                            return git2::Cred::username(username);
                        }
                    }

                    Some(Credentials::Password(ref password)) => {
                        if let Some(ref username) = password.username {
                            return git2::Cred::username(username);
                        }
                    }

                    None => return Err(git2::Error::from_str("Authentication is required")),
                }
            }

            if allowed.contains(git2::CredentialType::SSH_MEMORY) {
                if let Some(Credentials::SSH(ref ssh)) = self.config.credentials {
                    return git2::Cred::ssh_key_from_memory(
                        username.unwrap(),
                        ssh.public_key.as_ref().map(String::as_ref),
                        ssh.private_key.as_ref(),
                        ssh.passphrase.as_ref().map(String::as_ref),
                    );
                }
            }

            if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                if let Some(Credentials::Password(ref password)) = self.config.credentials {
                    return git2::Cred::userpass_plaintext(
                        username.unwrap(),
                        password.password.as_ref(),
                    );
                }
            }

            return Err(git2::Error::from_str("Unsupported authentication"));
        });

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

        return Ok(true);
    }

    #[allow(unused)]
    pub async fn last_checked(&self) -> Option<Instant> {
        let state = self.state.lock().await;
        return state.last_checked;
    }
}
