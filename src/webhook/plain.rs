use crate::config::PlainWebhook;
use crate::repo::Repo;
use rouille::Request;

pub fn handle(_repo: &Repo, _config: &PlainWebhook, request: &Request) -> Result<bool, String> {
    if request.method() != "POST" {
        return Err("Only POST ist allowed".to_owned());
    }

    return Ok(true);
}
