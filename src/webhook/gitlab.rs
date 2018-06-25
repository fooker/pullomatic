use config::GitLabWebhook;
use json;
use repo::Repo;
use rouille::Request;
use std::error::Error;
use std::io::Read;


pub fn handle(repo: &Repo, config: &GitLabWebhook, request: &Request) -> Result<bool, String> {
    if request.method() != "POST" {
        return Err("Only POST ist allowed".to_owned());
    }

    // Read the whole body
    let mut body = String::new();
    request.data().unwrap().take(1 * 1024 * 1024).read_to_string(&mut body).unwrap();

    // Check if the token matches
    if let Some(ref token) = config.token {
        if token != request.header("X-Gitlab-Token").ok_or("Token missing")? {
            return Err("Token mismatch".to_owned());
        }
    }

    // Only allow 'push' or 'ping' events
    let event = request.header("X-Gitlab-Event").ok_or("Not a GitLab webhook request")?;
    println!("[{}] Got GitLab event: {}", repo.name(), event);
    if event != "Push Hook" && event != "Push Event" {
        return Err(format!("Event not supported: {}", event));
    }

    // Parse the payload
    let payload = json::parse(&body).map_err(|e| format!("Invalid payload: {}", e.description()))?;

    // Check if push is for our remote branch
    println!("[{}] Got push event for '{}'", repo.name(), payload["ref"]);
    if config.check_branch.unwrap_or(true) &&
            payload["ref"].as_str() != Some(&repo.config().remote_ref()) {
        return Ok(false);
    }

    println!("[{}] Trigger update from hook", repo.name());
    return Ok(true);
}
