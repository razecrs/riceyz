//! Notion workspace search via the integration token.

fn title_of(page: &serde_json::Value) -> String {
    if let Some(props) = page["properties"].as_object() {
        for (_k, prop) in props {
            if prop["type"] == "title" {
                if let Some(arr) = prop["title"].as_array() {
                    return arr.iter().filter_map(|t| t["plain_text"].as_str()).collect();
                }
            }
        }
    }
    if let Some(arr) = page["title"].as_array() {
        return arr.iter().filter_map(|t| t["plain_text"].as_str()).collect();
    }
    String::new()
}

/// Search pages/databases shared with the integration -> (title, url).
pub fn search(token: &str, query: &str) -> Vec<(String, String)> {
    let req = format!(
        r#"{{"query":{},"page_size":8}}"#,
        serde_json::to_string(query).unwrap_or_else(|_| "\"\"".into())
    );
    let body = match ureq::post("https://api.notion.com/v1/search")
        .set("Authorization", &format!("Bearer {token}"))
        .set("Notion-Version", "2022-06-28")
        .set("Content-Type", "application/json")
        .send_string(&req)
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
    if let Some(results) = v["results"].as_array() {
        for r in results {
            let url = r["url"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let title = title_of(r);
            out.push((if title.is_empty() { "(untitled)".into() } else { title }, url));
        }
    }
    out
}
