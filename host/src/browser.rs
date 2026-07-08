//! Chromium browser bookmarks + history (Chrome / Edge / Brave).
use std::path::PathBuf;

/// Default profile dirs for installed Chromium browsers.
fn chromium_profiles() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    if let Ok(l) = std::env::var("LOCALAPPDATA") {
        for (name, sub) in [
            ("Chrome", r"Google\Chrome\User Data\Default"),
            ("Edge", r"Microsoft\Edge\User Data\Default"),
            ("Brave", r"BraveSoftware\Brave-Browser\User Data\Default"), // I cant lie i do not know the ones for opera gx etc Kindly add it and make a pr or wait for future updates
        ] {
            let p = PathBuf::from(&l).join(sub);
            if p.is_dir() {
                out.push((name.to_string(), p));
            }
        }
    }
    out
}

fn collect_bm(node: &serde_json::Value, out: &mut Vec<(String, String)>) {
    if node["type"] == "url" {
        if let (Some(name), Some(url)) = (node["name"].as_str(), node["url"].as_str()) {
            if !name.is_empty() {
                out.push((name.to_string(), url.to_string()));
            }
        }
        return;
    }
    if let Some(children) = node["children"].as_array() {
        for c in children {
            collect_bm(c, out);
        }
    }
    if let Some(roots) = node.as_object() {
        for (k, v) in roots {
            if matches!(k.as_str(), "children" | "type" | "name" | "url") {
                continue;
            }
            if v.is_object() {
                collect_bm(v, out);
            }
        }
    }
}

pub fn enum_bookmarks() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (_name, profile) in chromium_profiles() {
        if let Ok(txt) = std::fs::read_to_string(profile.join("Bookmarks")) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                collect_bm(&v["roots"], &mut out);
            }
        }
    }
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out.dedup_by(|a, b| a.1 == b.1);
    out.truncate(3000);
    out
}

pub fn enum_history() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (name, profile) in chromium_profiles() {
        let hist = profile.join("History");
        if !hist.is_file() {
            continue;
        }
        // The History DB is locked while the browser runs -> read a temp copy.
        let tmp = std::env::temp_dir().join(format!("bc_hist_{name}.db"));
        if std::fs::copy(&hist, &tmp).is_err() {
            continue;
        }
        if let Ok(conn) =
            rusqlite::Connection::open_with_flags(&tmp, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            if let Ok(mut stmt) = conn.prepare(
                "SELECT title, url FROM urls WHERE title != '' \
                 ORDER BY visit_count DESC, last_visit_time DESC LIMIT 500",
            ) {
                if let Ok(rows) =
                    stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                {
                    for row in rows.flatten() {
                        out.push(row);
                    }
                }
            }
        }
        let _ = std::fs::remove_file(&tmp);
    }
    out.dedup_by(|a, b| a.1 == b.1);
    out.truncate(1500);
    out
}
