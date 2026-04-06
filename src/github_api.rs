use serde_json::Value;
use std::env;

pub fn fetch_release_notes_from_github_api(
    owner: &str,
    repo: &str,
    version: &str,
) -> Option<String> {
    let token = env::var("GITHUB_TOKEN").ok();
    if token.is_none() {
        eprintln!("[DEBUG] No GITHUB_TOKEN set, skipping GitHub API for release notes");
        return None;
    }
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/releases/tags/{}",
        owner, repo, version
    );
    let client = reqwest::blocking::Client::new();
    let mut req = client
        .get(&api_url)
        .header("User-Agent", "cargo-whats-new")
        .header("Accept", "application/vnd.github+json");
    if let Some(token) = token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    match req.send() {
        Ok(resp) if resp.status().is_success() => match resp.text() {
            Ok(text) => {
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    if let Some(body) = json.get("body").and_then(|b| b.as_str()) {
                        if !body.trim().is_empty() {
                            eprintln!("[DEBUG] Got release notes from GitHub API");
                            return Some(body.trim().to_string());
                        }
                    }
                } else {
                    eprintln!("[DEBUG] Failed to parse GitHub API JSON");
                }
            }
            Err(e) => {
                eprintln!("[DEBUG] Failed to read GitHub API response text: {}", e);
            }
        },
        Ok(resp) => {
            eprintln!(
                "[DEBUG] GitHub API request failed: status {}",
                resp.status()
            );
        }
        Err(e) => {
            eprintln!("[DEBUG] GitHub API request error: {}", e);
        }
    }
    None
}
