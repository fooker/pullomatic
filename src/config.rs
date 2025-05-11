use serde::Deserialize;
use serde_humantime;
use serde_yaml;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
pub enum ConfigError {
    Parse(serde_yaml::Error),
    Io(io::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            ConfigError::Parse(ref err) => write!(f, "GIT error: {}", err),
            ConfigError::Io(ref err) => write!(f, "IO error: {}", err),
        }
    }
}

impl error::Error for ConfigError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            ConfigError::Parse(ref err) => Some(err),
            ConfigError::Io(ref err) => Some(err),
        }
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(err: serde_yaml::Error) -> Self {
        ConfigError::Parse(err)
    }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        ConfigError::Io(err)
    }
}

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
    pub path: String,

    pub remote_url: String,
    pub remote_branch: String,

    pub on_change: Option<String>,

    pub credentials: Option<Credentials>,

    pub interval: Option<Interval>,
    pub webhook: Option<Webhook>,
}

impl Config {
    pub fn load<P>(path: P) -> Result<HashMap<String, Self>, ConfigError>
    where
        P: AsRef<Path>,
    {
        let mut configs = HashMap::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let path = entry.path();

                configs.insert(
                    path.file_name().unwrap().to_str().unwrap().to_owned(),
                    Self::load_config(path)?,
                );
            }
        }

        return Ok(configs);
    }

    fn load_config<P>(path: P) -> Result<Self, ConfigError>
    where
        P: AsRef<Path>,
    {
        // FIXME: Specify interval as string (i.e. "5m")

        let mut input = String::new();
        fs::File::open(&path).and_then(|mut f| f.read_to_string(&mut input))?;

        let config = serde_yaml::from_str(&input)?;

        return Ok(config);
    }

    pub fn remote_ref(&self) -> String {
        return format!("refs/heads/{}", self.remote_branch);
    }
}
