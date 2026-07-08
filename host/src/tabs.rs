//! Browser open tabs bridge: a tiny localhost server the companion extension talks to.
//! POST /tabs  -> extension uploads the current tab list.
//! GET  /activate -> extension polls for a "switch to this tab" command.
use std::sync::{Arc, Mutex};

pub type Tabs = Arc<Mutex<Vec<(String, String, i64, i64)>>>; // (title, url, tabId, windowId)
pub type Pending = Arc<Mutex<Option<(i64, i64)>>>;

fn json(body: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let cors =
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();
    let ct =
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    tiny_http::Response::from_string(body).with_header(cors).with_header(ct)
}

pub fn serve(tabs: Tabs, pending: Pending) {
    let server = match tiny_http::Server::http("127.0.0.1:37421") {
        Ok(s) => s,
        Err(_) => return,
    };
    for mut req in server.incoming_requests() {
        let url = req.url().to_string();
        if url.starts_with("/tabs") {
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(arr) = v.as_array() {
                    let list: Vec<(String, String, i64, i64)> = arr
                        .iter()
                        .filter_map(|t| {
                            let u = t["url"].as_str()?.to_string();
                            if u.is_empty() {
                                return None;
                            }
                            Some((
                                t["title"].as_str().unwrap_or("").to_string(),
                                u,
                                t["id"].as_i64().unwrap_or(-1),
                                t["windowId"].as_i64().unwrap_or(-1),
                            ))
                        })
                        .collect();
                    if let Ok(mut g) = tabs.lock() {
                        *g = list;
                    }
                }
            }
            let _ = req.respond(json("{\"ok\":true}"));
        } else if url.starts_with("/activate") {
            let cmd = pending.lock().ok().and_then(|mut g| g.take());
            let body = match cmd {
                Some((t, w)) => format!(r#"{{"tabId":{t},"windowId":{w}}}"#),
                None => "{}".to_string(),
            };
            let _ = req.respond(json(&body));
        } else {
            let _ = req.respond(json("{}"));
        }
    }
}
