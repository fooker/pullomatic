use config::Config;
use git2;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;


struct RepoState {
    last_updated: Option<Instant>,
    last_changed: Option<Instant>,
}

pub struct Repo {
    name: String,
    config: Config,

    state: Mutex<RepoState>,
}

pub struct Changes {
    pub prev_id: Option<git2::Oid>,
    pub curr_id: git2::Oid,
}

#[derive(Debug)]
pub enum UpdateError {
    Git(git2::Error),
    Io(io::Error),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            UpdateError::Git(ref err) => write!(f, "GIT error: {}", err),
            UpdateError::Io(ref err) => write!(f, "IO error: {}", err),
        }
    }
}

impl error::Error for UpdateError {
    fn description(&self) -> &str {
        match *self {
            UpdateError::Git(ref err) => err.description(),
            UpdateError::Io(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            UpdateError::Git(ref err) => Some(err),
            UpdateError::Io(ref err) => Some(err),
        }
    }
}

impl From<git2::Error> for UpdateError {
    fn from(err: git2::Error) -> Self { UpdateError::Git(err) }
}

impl From<io::Error> for UpdateError {
    fn from(err: io::Error) -> Self { UpdateError::Io(err) }
}

impl Repo {
    pub fn new(name: String, config: Config) -> Self {
        return Self {
            name,
            config,

            state: Mutex::new(RepoState {
                last_changed: None,
                last_updated: None,
            }),
        };
    }

    pub fn update(&self) -> Result<Option<Changes>, UpdateError> {
        let now = Some(Instant::now());

        self.state.lock().unwrap().last_updated = now;

        let path = Path::new(&self.config.path);

        let repository: git2::Repository;
        if path.exists() {
            println!("[{}] Using existing repository", self.name);

            // Open the repo or give up
            repository = git2::Repository::open(path)?;
        } else {
            println!("[{}] Initialized new repository", self.name);

            // Create the directory and init the repo
            fs::create_dir_all(path)?;
            repository = git2::Repository::init(path)?;
        }

        let mut remote = repository.remote_anonymous(&self.config.remote_url)?;

        let mut remote_cb = git2::RemoteCallbacks::new();
        remote_cb.credentials(|url, username, allowed| {
            // FIXME: Implement in-memory keys

            println!("[{}] cred: url = {:?}", self.name, url);
            println!("[{}] cred: username = {:?}", self.name, username);
            println!("[{}] cred: allowed = {:?}", self.name, allowed);

            return git2::Cred::ssh_key(username.unwrap(), None, Path::new(""), None);
        });

        println!("[{}] Fetching data from remote", self.name);
        remote.fetch(&[&format!("+refs/heads/{}:refs/pullomatic", self.config.remote_branch)],
                     Some(git2::FetchOptions::new()
                             .prune(git2::FetchPrune::On)
                             .remote_callbacks(remote_cb)),
                     None)?;
        println!("[{}] Fetched data from remote", self.name);

        repository.find_reference("HEAD")?;
        let latest_obj = repository.revparse_single("HEAD").ok();
        let remote_obj = repository.revparse_single("refs/pullomatic")?;

        if let Some(ref latest_obj) = latest_obj {
            if latest_obj.id() == remote_obj.id() {
                println!("[{}] Already up to date", self.name);
                return Ok(None);
            }
        }

        repository.reset(&remote_obj,
                         git2::ResetType::Hard,
                         Some(git2::build::CheckoutBuilder::new()
                                 .force()
                                 .remove_untracked(true)))?;

        println!("[{}] Updated to {}", self.name, remote_obj.id());
        self.state.lock().unwrap().last_changed = now;
        return Ok(Some(Changes {
            prev_id: latest_obj.map(|ref obj| obj.id()),
            curr_id: remote_obj.id(),
        }));
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn config(&self) -> &Config { &self.config }

    pub fn last_updated(&self) -> Option<Instant> { self.state.lock().unwrap().last_updated }
    pub fn last_changed(&self) -> Option<Instant> { self.state.lock().unwrap().last_changed }
}
