//! GitHub via the authenticated `gh` CLI: your repos + unread notifications.
use std::process::Command;

fn gh_json(args: &[&str]) -> Option<serde_json::Value> {
    let out = Command::new("gh").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

/// Your repos -> (nameWithOwner, url).
pub fn repos() -> Vec<(String, String)> {
    let v = match gh_json(&["repo", "list", "--limit", "300", "--json", "nameWithOwner,url"]) {
        Some(v) => v,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    if let Some(arr) = v.as_array() {
        for r in arr {
            if let (Some(n), Some(u)) = (r["nameWithOwner"].as_str(), r["url"].as_str()) {
                out.push((n.to_string(), u.to_string()));
            }
        }
    }
    out
}

/// Unread notifications -> (title, repo-url).
pub fn notifications() -> Vec<(String, String)> {
    let v = match gh_json(&["api", "/notifications"]) {
        Some(v) => v,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    if let Some(arr) = v.as_array() {
        for n in arr {
            let title = n["subject"]["title"].as_str().unwrap_or("");
            let repo = n["repository"]["full_name"].as_str().unwrap_or("");
            if !title.is_empty() {
                out.push((title.to_string(), format!("https://github.com/{repo}")));
            }
        }
    }
    out
}
