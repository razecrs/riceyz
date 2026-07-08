//! Spotify search via app (client-credentials) token. Playback needs user OAuth (later).
use std::sync::Mutex;
use std::time::Instant;

use base64::Engine;

static TOKEN: Mutex<Option<(String, Instant)>> = Mutex::new(None);

fn app_token(id: &str, secret: &str) -> Option<String> {
    if let Ok(g) = TOKEN.lock() {
        if let Some((t, at)) = g.as_ref() {
            if at.elapsed().as_secs() < 3300 {
                return Some(t.clone());
            }
        }
    }
    let auth = base64::engine::general_purpose::STANDARD.encode(format!("{id}:{secret}"));
    let body = ureq::post("https://accounts.spotify.com/api/token")
        .set("Authorization", &format!("Basic {auth}"))
        .send_form(&[("grant_type", "client_credentials")])
        .ok()?
        .into_string()
        .ok()?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    let tok = v["access_token"].as_str()?.to_string();
    if let Ok(mut g) = TOKEN.lock() {
        *g = Some((tok.clone(), Instant::now()));
    }
    Some(tok)
}

/// Search tracks -> (name, artist, url).
pub fn search(id: &str, secret: &str, query: &str) -> Vec<(String, String, String)> {
    let token = match app_token(id, secret) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let url = format!(
        "https://api.spotify.com/v1/search?type=track&limit=6&q={}",
        crate::net::urlencode(query)
    );
    let body = match ureq::get(&url)
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .ok()
        .and_then(|r| r.into_string().ok())
    {
        Some(b) => b,
        None => return Vec::new(),
    };
    let v: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    if let Some(items) = v["tracks"]["items"].as_array() {
        for t in items {
            let name = t["name"].as_str().unwrap_or("").to_string();
            let artist = t["artists"][0]["name"].as_str().unwrap_or("").to_string();
            let url = t["external_urls"]["spotify"].as_str().unwrap_or("").to_string();
            if !name.is_empty() {
                out.push((name, artist, url));
            }
        }
    }
    out
}
