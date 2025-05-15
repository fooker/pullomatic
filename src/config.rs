use anyhow::{Context, Result};
use serde::Deserialize;
use serde_humantime;
use serde_yaml;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, Deserialize)]
pub struct SSHCredentials {
    pub username: Option<String>,

    pub public_key: Option<String>,
    pub private_key: String,

    pub passphrase: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PasswordCredentials {
    pub username: Option<String>,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Credentials {
    SSH(SSHCredentials),
    Password(PasswordCredentials),
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlainWebhook {}

#[derive(Clone, Debug, Deserialize)]
pub struct GitHubWebhook {
    pub secret: Option<String>,
    pub check_branch: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GitLabWebhook {
    pub token: Option<String>,
    pub check_branch: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum Webhook {
    Plain(PlainWebhook),
    GitHub(GitHubWebhook),
    GitLab(GitLabWebhook),
}

#[derive(Clone, Debug, Deserialize)]
pub struct Interval {
    #[serde(with = "serde_humantime")]
    pub interval: Duration,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub path: PathBuf,

    pub remote_url: String,
    pub remote_branch: String,

    pub on_change: Option<String>,

    pub credentials: Option<Credentials>,

    pub interval: Option<Interval>,
    pub webhook: Option<Webhook>,
}

impl Config {
    pub async fn load(path: &Path) -> Result<HashMap<String, Self>> {
        let mut configs = HashMap::new();

        if !path.exists() {
            anyhow::bail!("Config directory does not exist: {}", path.display());
        }

        let mut dir = tokio::fs::read_dir(path)
            .await
            .with_context(|| format!("Failed to read config directory: {}", path.display()))?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();

            // TODO: Can we do this better?
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();

            let config = Self::load_config(&path)
                .await
                .with_context(|| format!("Failed to load config file: {}", path.display()))?;

            configs.insert(name, config);
        }

        return Ok(configs);
    }

    async fn load_config(path: &Path) -> Result<Self> {
        // FIXME: Specify interval as string (i.e. "5m")

        let input = tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config = serde_yaml::from_str(&input)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        return Ok(config);
    }

    pub fn remote_ref(&self) -> String {
        return format!("refs/heads/{}", self.remote_branch);
    }
}
