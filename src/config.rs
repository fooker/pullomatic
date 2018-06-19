use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::time::Duration;
use toml;


#[derive(Debug)]
pub enum ConfigError {
    Parse(toml::de::Error),
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
    fn description(&self) -> &str {
        match *self {
            ConfigError::Parse(ref err) => err.description(),
            ConfigError::Io(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ConfigError::Parse(ref err) => Some(err),
            ConfigError::Io(ref err) => Some(err),
        }
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self { ConfigError::Parse(err) }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self { ConfigError::Io(err) }
}


#[derive(Clone, Debug, Deserialize)]
pub struct SSHCredentials {
    pub public_key: String,
    pub private_key: String,

    pub passphrase: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub enum WebhookProvider {
    GitHub,
    GitLab,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Webhook {
//    pub provider: WebhookProvider,

    pub secret: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub path: String,

    pub remote_url: String,
    pub remote_branch: String,

    pub on_change: Option<String>,

    pub ssh: Option<SSHCredentials>,

    pub interval: Option<Duration>,
    pub webhook: Option<Webhook>,

}

impl Config {
    pub fn load<P>(path: P) -> Result<HashMap<String, Self>, ConfigError> where P: AsRef<Path> {
        let mut configs = HashMap::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let path = entry.path();

                configs.insert(path.file_name().unwrap().to_str().unwrap().to_owned(),
                               Self::load_config(path)?);
            }
        }

        return Ok(configs);
    }

    fn load_config<P>(path: P) -> Result<Self, ConfigError> where P: AsRef<Path> {
        // FIXME: Specify interval as string (i.e. "5m")

        let mut input = String::new();
        fs::File::open(&path).and_then(|mut f| {
            f.read_to_string(&mut input)
        })?;

        let config = toml::from_str(&input)?;

        return Ok(config);
    }
}
