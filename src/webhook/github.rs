use config::GitHubWebhook;
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::sha1::Sha1;
use hex;
use json;
use repo::Repo;
use rouille::Request;
use std::io::Read;


pub fn handle(repo: &Repo, config: &GitHubWebhook, request: &Request) -> Result<bool, String> {
    if request.method() != "POST" {
        return Err("Only POST ist allowed".to_owned());
    }

    // Read the whole body
    let mut body = String::new();
    request.data().unwrap().take(1 * 1024 * 1024).read_to_string(&mut body).unwrap();

    // Check if the signature matches the secret
    if let Some(ref secret) = config.secret {
        let signature = request.header("X-Hub-Signature").ok_or("Signature missing")?;

        let mut hmac = Hmac::new(Sha1::new(), secret.as_bytes());
        hmac.input(body.as_bytes());

        if signature != format!("sha1={}", hex::encode(hmac.result().code())) {
            return Err("Signature mismatch".to_owned());
        }
    }

    // Only allow 'push' or 'ping' events
    let event = request.header("X-GitHub-Event").ok_or("Not a GitHub webhook request")?;
    println!("[{}] Got GitHub event: {}", repo.name(), event);
    if event == "ping" {
        return Ok(false);
    } else if event != "push" {
        return Err(format!("Event not supported: {}", event));
    }

    // Parse the payload
    let payload = json::parse(&body).map_err(|e| format!("Invalid payload: {}", e))?;

    // Check if push is for our remote branch
    println!("[{}] Got push event for '{}'", repo.name(), payload["ref"]);
    if config.check_branch.unwrap_or(true) &&
            payload["ref"].as_str() != Some(&repo.config().remote_ref()) {
        return Ok(false);
    }

    println!("[{}] Trigger update from hook", repo.name());
    return Ok(true);
}

