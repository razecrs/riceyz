//! Installed apps, Steam games, Obsidian notes; app + shell launching; brightness.
use std::path::PathBuf;

/// Enumerate installed apps from the Start Menu (.lnk files) -> (name, path).
pub fn enum_apps() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("lnk")) == Some(true) {
                    if let Some(name) = p.file_stem().and_then(|x| x.to_str()) {
                        out.push((name.to_string(), p.to_string_lossy().to_string()));
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    let roots = [
        std::env::var("ProgramData").ok().map(|p| format!(r"{p}\Microsoft\Windows\Start Menu\Programs")),
        std::env::var("APPDATA").ok().map(|p| format!(r"{p}\Microsoft\Windows\Start Menu\Programs")),
    ];
    for r in roots.into_iter().flatten() {
        walk(std::path::Path::new(&r), &mut out);
    }
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out.dedup_by(|a, b| a.0.eq_ignore_ascii_case(&b.0));
    out
}

/// Launch a well-known app from a dashboard tab.
pub fn run_app(name: &str) {
    let cmd = match name {
        "Edge" => "start Edge/chrome/etc",
        "terminal" => "start wt",
        "files" => "start explorer",
        "steam" => "start steam://open/main",
        "code" => "start code-editor/vs-code/etc",
        _ => return,
    };
    let _ = std::process::Command::new("cmd").args(["/C", cmd]).spawn();
}

/// Run a shell command in a visible window; admin variants trigger UAC.
pub fn run_shell(mode: &str, cmd: &str) {
    use std::process::Command;
    let _ = match mode {
        "cmd" => Command::new("cmd").args(["/C", "start", "cmd", "/K", cmd]).spawn(),
        "ps" => Command::new("cmd")
            .args(["/C", "start", "powershell", "-NoExit", "-Command", cmd])
            .spawn(),
        "cmdadmin" => Command::new("powershell")
            .args(["-Command", &format!("Start-Process cmd -ArgumentList '/K {cmd}' -Verb RunAs")])
            .spawn(),
        "psadmin" => Command::new("powershell")
            .args(["-Command", &format!("Start-Process powershell -ArgumentList '-NoExit -Command {cmd}' -Verb RunAs")])
            .spawn(),
        _ => return,
    };
}

/// Laptop-panel brightness via WMI (external monitors would need DDC/CI later).
pub fn set_brightness(pct: u32) {
    let p = pct.min(100);
    let script =
        format!("(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1,{p})");
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn();
}

fn steam_root() -> Option<PathBuf> {
    for p in [
        r"C:\Program Files (x86)\Steam",
        r"C:\Program Files\Steam",
        r"D:\Steam",
        r"E:\Steam",  // Add here per your drive
    ] {
        let pb = PathBuf::from(p);
        if pb.join("steamapps").is_dir() {
            return Some(pb);
        }
    }
    None
}

/// Pull a `"key"  "value"` pair out of a Valve VDF/ACF line.
fn acf_val(txt: &str, key: &str) -> Option<String> {
    for line in txt.lines() {
        let parts: Vec<&str> = line.split('"').collect();
        if parts.len() >= 4 && parts[1] == key {
            return Some(parts[3].to_string());
        }
    }
    None
}

/// Installed Steam games across all library folders -> (appid, name).
pub fn enum_steam() -> Vec<(String, String)> {
    let root = match steam_root() {
        Some(r) => r,
        None => return Vec::new(),
    };
    let mut libs = vec![root.join("steamapps")];
    if let Ok(txt) = std::fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        for line in txt.lines() {
            if line.contains("\"path\"") {
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() >= 4 {
                    let p = parts[parts.len() - 2].replace("\\\\", "\\");
                    let lib = PathBuf::from(p).join("steamapps");
                    if lib.is_dir() && !libs.contains(&lib) {
                        libs.push(lib);
                    }
                }
            }
        }
    }
    let mut games: Vec<(String, String)> = Vec::new();
    for lib in libs {
        if let Ok(rd) = std::fs::read_dir(&lib) {
            for e in rd.flatten() {
                let name = e.file_name();
                let n = name.to_string_lossy();
                if n.starts_with("appmanifest_") && n.ends_with(".acf") {
                    if let Ok(txt) = std::fs::read_to_string(e.path()) {
                        if let (Some(a), Some(nm)) = (acf_val(&txt, "appid"), acf_val(&txt, "name")) {
                            games.push((a, nm));
                        }
                    }
                }
            }
        }
    }
    games.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    games.dedup_by(|a, b| a.0 == b.0);
    games
}

fn walk_md(dir: &std::path::Path, out: &mut Vec<(String, String)>, depth: usize) {
    if depth > 8 || out.len() > 2000 {
        return;
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let hidden = p.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with('.')).unwrap_or(false);
                if !hidden {
                    walk_md(&p, out, depth + 1);
                }
            } else if p.extension().and_then(|x| x.to_str()) == Some("md") {
                if let Some(name) = p.file_stem().and_then(|x| x.to_str()) {
                    out.push((name.to_string(), p.to_string_lossy().to_string()));
                }
            }
        }
    }
}

/// Obsidian notes across all vaults (auto-detected from obsidian.json) -> (name, path).
pub fn enum_obsidian() -> Vec<(String, String)> {
    let cfg = match std::env::var("APPDATA") {
        Ok(a) => PathBuf::from(a).join("obsidian").join("obsidian.json"),
        Err(_) => return Vec::new(),
    };
    let txt = match std::fs::read_to_string(&cfg) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let v: serde_json::Value = match serde_json::from_str(&txt) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut notes = Vec::new();
    if let Some(vaults) = v["vaults"].as_object() {
        for (_id, vault) in vaults {
            if let Some(path) = vault["path"].as_str() {
                walk_md(std::path::Path::new(path), &mut notes, 0);
            }
        }
    }
    notes.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    notes.dedup_by(|a, b| a.1 == b.1);
    notes.truncate(2000);
    notes
}
