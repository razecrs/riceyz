//! Plugin host: external processes provide launcher results over line-delimited JSON stdio.
//!
//! Each `plugins/<name>/plugin.json` = { "name", "trigger", "command" }.
//! Host -> plugin (stdin):  {"query":"..."}\n
//! Plugin -> host (stdout): {"results":[{"title","subtitle","action"}]}\n
//! `action` is any launcher IPC verb (open:/shell:/launch:/copy:/sys:...), run on select.
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

struct Plugin {
    trigger: String,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    _child: Child,
}

impl Plugin {
    fn call(&mut self, q: &str) -> Option<Vec<(String, String, String)>> {
        let req = format!("{}\n", serde_json::json!({ "query": q }));
        self.stdin.write_all(req.as_bytes()).ok()?;
        self.stdin.flush().ok()?;
        let mut line = String::new();
        self.stdout.read_line(&mut line).ok()?;
        let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
        let arr = v["results"].as_array()?;
        Some(
            arr.iter()
                .filter_map(|r| {
                    let title = r["title"].as_str()?.to_string();
                    Some((
                        title,
                        r["subtitle"].as_str().unwrap_or("").to_string(),
                        r["action"].as_str().unwrap_or("").to_string(),
                    ))
                })
                .collect(),
        )
    }
}

pub struct Host {
    plugins: Vec<Mutex<Plugin>>,
    names: Vec<(String, String)>, // (name, trigger)
}

impl Host {
    /// Discover + launch every plugin under `dir`.
    pub fn load(dir: &Path) -> Host {
        let mut plugins = Vec::new();
        let mut names = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let manifest = e.path().join("plugin.json");
                let txt = match std::fs::read_to_string(&manifest) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let m: serde_json::Value = match serde_json::from_str(&txt) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let name = m["name"].as_str().unwrap_or("plugin").to_string();
                let trigger = m["trigger"].as_str().unwrap_or("").to_string();
                let cmd = match m["command"].as_str() {
                    Some(c) if !c.is_empty() => c.to_string(),
                    _ => continue,
                };
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                let mut child = match Command::new(parts[0])
                    .args(&parts[1..])
                    .current_dir(e.path())
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
                    continue;
                };
                names.push((name, trigger.clone()));
                plugins.push(Mutex::new(Plugin {
                    trigger,
                    stdin,
                    stdout: BufReader::new(stdout),
                    _child: child,
                }));
            }
        }
        Host { plugins, names }
    }

    /// Query every plugin whose trigger matches -> (title, subtitle, action).
    pub fn query(&self, q: &str) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for pm in &self.plugins {
            if let Ok(mut p) = pm.lock() {
                if !p.trigger.is_empty() && !q.starts_with(&p.trigger) {
                    continue;
                }
                let sub = if p.trigger.is_empty() {
                    q
                } else {
                    q[p.trigger.len()..].trim()
                };
                if let Some(res) = p.call(sub) {
                    out.extend(res);
                }
            }
        }
        out
    }

    /// Loaded plugins -> (name, trigger).
    pub fn list(&self) -> &[(String, String)] {
        &self.names
    }
}
