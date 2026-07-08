//! Batcave a Rust desktop shell. (Well you can add your own styles)
//!
//! One process drives three WebView2 surfaces (wallpaper dashboard, launcher, toast),
//! reparents the dashboard onto the desktop layer, and wires them to Win32 + WinRT:
//! live stats, an `Alt+Space` launcher, MFT file search, an audio visualizer, and
//! real Windows notifications replayed as a batarang. The UI is plain HTML/CSS/JS;
//! `main` is mostly window setup, the event loop, and IPC routing between the two.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unsafe_op_in_unsafe_fn)] // lots of Win32 lives in `unsafe fn`s; kept terse on purpose

use std::borrow::Cow;
use std::path::PathBuf;
use std::time::Duration;

use tao::{
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::windows::WindowExtWindows,
    window::WindowBuilder,
};
use wry::WebViewBuilder;

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Mutex};

use windows::core::{w, BOOL, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, TRUE, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, EnumChildWindows, EnumWindows, FindWindowExW, FindWindowW, GetClassNameW,
    GetForegroundWindow, GetParent, GetWindowThreadProcessId, PostMessageW, SendMessageTimeoutW,
    SetParent, SetWindowsHookExW, ShowWindow, WindowFromPoint, MSLLHOOKSTRUCT, SMTO_NORMAL, SW_HIDE,
    WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
};

mod apps;
mod audio;
mod browser;
mod github;
mod icons;
mod media;
mod mft;
mod net;
mod notifications;
mod notion;
mod plugins;
mod proc;
mod specs;
mod spotify;
mod tabs;
mod viz;
use apps::{enum_apps, enum_obsidian, enum_steam, run_app, run_shell, set_brightness};
use audio::{process_name, set_app_volume, set_master_volume, set_mute};
use media::{media_command, now_playing};

/// Hides the taskbar (main + secondary). Safe to call as much as you want.
fn hide_taskbar() {
    unsafe {
        for cls in [w!("Shell_TrayWnd"), w!("Shell_SecondaryTrayWnd")] {
            if let Ok(h) = FindWindowW(cls, PCWSTR::null()) {
                if !h.0.is_null() {
                    let _ = ShowWindow(h, SW_HIDE);
                }
            }
        }
    }
}

// HWNDs the mouse hook needs (kept as isize so they fit in atomics).
static TARGET: AtomicIsize = AtomicIsize::new(0); // WebView2's Chromium input window
static SELFW: AtomicIsize = AtomicIsize::new(0); // our wallpaper top-level window

/// Window-class name of `hwnd`.
unsafe fn class_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = GetClassNameW(hwnd, &mut buf);
    String::from_utf16_lossy(&buf[..n as usize])
}

/// Is the cursor over our wallpaper (i.e. the desktop is showing, no app window on top)?
unsafe fn on_wallpaper(pt: POINT) -> bool {
    let w = WindowFromPoint(pt);
    if w.0.is_null() {
        return false;
    }
    let selfw = SELFW.load(Ordering::Relaxed);
    let mut cur = w;
    for _ in 0..6 {
        if cur.0 as isize == selfw {
            return true;
        }
        let c = class_of(cur);
        if c == "WorkerW" || c == "Progman" || c == "SHELLDLL_DefView" || c == "SysListView32" {
            return true;
        }
        match GetParent(cur) {
            Ok(p) if !p.0.is_null() => cur = p,
            _ => break,
        }
    }
    false
}

/// Low-level mouse hook: when the click lands on the wallpaper, forward it into WebView2
/// so the desktop wallpaper is actually interactive.
unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let pt = ms.pt;
        if on_wallpaper(pt) {
            let t = TARGET.load(Ordering::Relaxed);
            if t != 0 {
                let target = HWND(t as *mut std::ffi::c_void);
                let mut cp = pt;
                let _ = ScreenToClient(target, &mut cp);
                let lp = LPARAM(((cp.y << 16) | (cp.x & 0xFFFF)) as isize);
                match wparam.0 as u32 {
                    WM_LBUTTONDOWN => {
                        let _ = PostMessageW(Some(target), WM_LBUTTONDOWN, WPARAM(1), lp);
                    }
                    WM_LBUTTONUP => {
                        let _ = PostMessageW(Some(target), WM_LBUTTONUP, WPARAM(0), lp);
                    }
                    WM_MOUSEMOVE => {
                        let _ = PostMessageW(Some(target), WM_MOUSEMOVE, WPARAM(0), lp);
                    }
                    _ => {}
                }
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// `EnumChildWindows` callback: find the Chromium render window inside our WebView2
/// (the one that actually receives mouse input) and hand it back via the `LPARAM` pointer.
unsafe extern "system" fn find_wv(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = lparam.0 as *mut HWND;
    if class_of(hwnd) == "Chrome_RenderWidgetHostHWND" {
        *out = hwnd;
        return BOOL(0); // stop enumerating
    }
    TRUE
}

/// Messages background threads and the webview IPC handlers post into the event loop.
enum UserEvent {
    Stats(String),
    NowPlaying(String),
    Viz(String),
    Reload,
    ToggleLauncher,
    HideLauncher,
    ResizeLauncher(f64),
    Notif(String),
    ShowToast(String),
    HideToast,
    EvalLauncher(String),
}

/// Is this foreground window the desktop itself? (Decides batarang cinematic vs. app toast.)
fn foreground_is_desktop(h: isize) -> bool {
    if h == 0 {
        return true;
    }
    let c = unsafe { class_of(HWND(h as *mut std::ffi::c_void)) };
    c == "Progman" || c == "WorkerW" || c == "SHELLDLL_DefView" || c == "SysListView32"
}

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};

/// Tiny JSON string escape, just enough to drop random text into the evaluate_script
/// calls we push to the webviews.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
}

/// `EnumWindows` callback: find the `WorkerW` that sits behind the desktop icons and
/// return it via the `LPARAM` pointer.
unsafe extern "system" fn enum_proc(top: HWND, lparam: LPARAM) -> BOOL {
    let out = lparam.0 as *mut HWND;
    let shell = FindWindowExW(Some(top), None, w!("SHELLDLL_DefView"), PCWSTR::null());
    if let Ok(sh) = shell {
        if !sh.0.is_null() {
            if let Ok(worker) = FindWindowExW(None, Some(top), w!("WorkerW"), PCWSTR::null()) {
                if !out.is_null() {
                    *out = worker;
                }
            }
        }
    }
    TRUE
}

/// Resolve the window to reparent the wallpaper onto: the `WorkerW` behind the icons,
/// or `Progman` itself on Windows 11 (which usually has no separate `WorkerW`).
fn desktop_layer() -> Option<HWND> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), PCWSTR::null()).ok()?;
        // Ask Progman to spawn a WorkerW behind the icons (Win10 / some Win11).
        let mut res: usize = 0;
        let _ = SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0xD),
            LPARAM(0x1),
            SMTO_NORMAL,
            1000,
            Some(&mut res as *mut usize),
        );
        let mut worker: HWND = HWND(std::ptr::null_mut());
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut worker as *mut HWND as isize));
        if !worker.0.is_null() {
            Some(worker)
        } else {
            Some(progman) // Win11 usually has no WorkerW, so just parent onto Progman
        }
    }
}

fn main() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // Which folder to serve the UI from. Defaults to the install folder, but point
    // config.json "ui" at any skin folder (that follows PROTOCOL.md) to run a custom UI.
    let ui_dir = std::fs::read_to_string(r"R:\Projects\batcave-dashboard\config.json")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["ui"].as_str().map(PathBuf::from))
        .filter(|p| p.join("index.html").exists())
        .unwrap_or_else(|| PathBuf::from(r"R:\Projects\batcave-dashboard"));

    // Detect this machine's hardware and refresh specs.json inside the active UI folder.
    {
        let p = ui_dir.join("specs.json").to_string_lossy().into_owned();
        std::thread::spawn(move || specs::refresh(&p));
    }

    // The dashboard is opaque (WebView2 transparency renders white here), so drop the current
    // Windows wallpaper into the UI folder as its background instead of plain black.
    if let Ok(appdata) = std::env::var("APPDATA") {
        let src = format!(r"{appdata}\Microsoft\Windows\Themes\TranscodedWallpaper");
        let _ = std::fs::copy(src, ui_dir.join("wallpaper.png"));
    }

    let monitor = event_loop.primary_monitor().expect("no primary monitor");
    let sz: PhysicalSize<u32> = monitor.size();

    let window = WindowBuilder::new()
        .with_decorations(false)
        .with_inner_size(sz)
        .build(&event_loop)
        .expect("window build failed");
    window.set_outer_position(PhysicalPosition::new(0, 0));

    // Reparent onto the desktop wallpaper layer.
    let hwnd = HWND(window.hwnd() as *mut std::ffi::c_void);
    if let Some(layer) = desktop_layer() {
        unsafe {
            let _ = SetParent(hwnd, Some(layer));
        }
    }
    // Re-anchor to top-left of the new parent.
    window.set_outer_position(PhysicalPosition::new(0, 0));

    // Kill the Windows taskbar (kept down by the loop below).
    hide_taskbar();

    // Serve the web assets folder through a custom protocol.
    let root = ui_dir.clone();
    let proxy_d = event_loop.create_proxy();
    // One shared WebView2 environment for ALL surfaces -> a single browser+GPU process set
    // instead of a full Chromium stack per webview (major RAM saving + shared localStorage).
    let mut web_ctx = wry::WebContext::new(Some(PathBuf::from(
        r"R:\Projects\batcave-dashboard\host\.webview2",
    )));
    let webview = WebViewBuilder::new_with_web_context(&mut web_ctx)
        .with_transparent(false)
        .with_custom_protocol("bat".into(), move |_id, request| {
            let p = request.uri().path().trim_start_matches('/');
            let rel = if p.is_empty() { "index.html" } else { p };
            let file = root.join(rel);
            let body = std::fs::read(&file).unwrap_or_default();
            let mime = match file.extension().and_then(|e| e.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("js") => "text/javascript; charset=utf-8",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("json") => "application/json",
                _ => "application/octet-stream",
            };
            wry::http::Response::builder()
                .header("Content-Type", mime)
                .header("Access-Control-Allow-Origin", "*")
                .header("Cache-Control", "no-store, must-revalidate")
                .body(Cow::from(body))
                .unwrap()
        })
        .with_ipc_handler(move |req| {
            let body = req.body().to_string();
            if let Some(cmd) = body.strip_prefix("media:") {
                let c = cmd.to_string();
                std::thread::spawn(move || media_command(&c));
            } else if let Some(msg) = body.strip_prefix("showtoast:") {
                let _ = proxy_d.send_event(UserEvent::ShowToast(msg.to_string()));
            } else if body == "hidetoast" {
                let _ = proxy_d.send_event(UserEvent::HideToast);
            } else if let Some(app) = body.strip_prefix("run:") {
                let a = app.to_string();
                std::thread::spawn(move || run_app(&a));
            } else if let Some(url) = body.strip_prefix("open:") {
                let u = url.to_string();
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start", "", &u])
                        .spawn();
                });
            } else if let Some(act) = body.strip_prefix("power:") {
                let cmd: Vec<&str> = match act {
                    "shutdown" => vec!["shutdown", "/s", "/t", "0"],
                    "restart" => vec!["shutdown", "/r", "/t", "0"],
                    "signout" => vec!["shutdown", "/l"],
                    "lock" => vec!["rundll32.exe", "user32.dll,LockWorkStation"],
                    "sleep" => vec!["rundll32.exe", "powrprof.dll,SetSuspendState", "0,1,0"],
                    _ => vec![],
                };
                if !cmd.is_empty() {
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new(cmd[0]).args(&cmd[1..]).spawn();
                    });
                }
            } else if body == "reload" {
                let _ = proxy_d.send_event(UserEvent::Reload);
            }
        })
        .with_url("bat://localhost/index.html")
        .build(&window)?;

    // Poll real stats off-thread, push into the event loop.
    std::thread::spawn(move || {
        let mut sys = sysinfo::System::new_all();
        loop {
            hide_taskbar();
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            let cpu = sys.global_cpu_usage();
            let total = sys.total_memory().max(1) as f64;
            let ram = sys.used_memory() as f64 / total * 100.0;
            let json = format!(r#"{{"cpu":{:.1},"ram":{:.1}}}"#, cpu, ram);
            let _ = proxy.send_event(UserEvent::Stats(json));

            let np = match now_playing() {
                Some((t, a, p)) => format!(
                    r#"{{"title":"{}","artist":"{}","playing":{}}}"#,
                    esc(&t), esc(&a), p
                ),
                None => r#"{"title":"","artist":"","playing":false}"#.to_string(),
            };
            let _ = proxy.send_event(UserEvent::NowPlaying(np));

            std::thread::sleep(Duration::from_millis(1500));
        }
    });

    // Surface real Windows notifications as batarangs (one-time access prompt).
    let proxy_n = event_loop.create_proxy();
    std::thread::spawn(move || {
        if !notifications::request_access() {
            return;
        }
        notifications::set_native_toasts(false); // hide the OS banner; we render our own
        let mut seen = std::collections::HashSet::new();
        let _ = notifications::poll(&mut seen); // prime: don't fire for pre-existing notifs
        loop {
            std::thread::sleep(Duration::from_secs(2));
            for (app, text) in notifications::poll(&mut seen) {
                let msg = match (app.is_empty(), text.is_empty()) {
                    (false, false) => format!("{app} · {text}"),
                    (false, true) => app,
                    (true, false) => text,
                    _ => continue,
                };
                let _ = proxy_n.send_event(UserEvent::Notif(msg));
            }
        }
    });

    // Audio visualizer: WASAPI loopback -> FFT bars, throttled to ~30fps -> dashboard.
    {
        let pv = event_loop.create_proxy();
        let mut last = std::time::Instant::now();
        viz::run(move |bars| {
            if last.elapsed().as_millis() < 33 {
                return;
            }
            last = std::time::Instant::now();
            let json = bars.iter().map(|b| format!("{b:.3}")).collect::<Vec<_>>().join(",");
            let _ = pv.send_event(UserEvent::Viz(json));
        });
    }

    // ---- Launcher (surface #2): hidden full-screen overlay, Alt+Space toggles it ----
    let launcher_win = WindowBuilder::new()
        .with_decorations(false)
        .with_transparent(false)
        .with_visible(false)
        .with_always_on_top(true)
        .with_inner_size(LogicalSize::new(680.0, 120.0))
        .build(&event_loop)
        .expect("launcher window failed");
    let launcher_id = launcher_win.id();
    // Anchor horizontally centered, ~16% from the top of the screen.
    {
        let scale = launcher_win.scale_factor();
        let x = ((sz.width as f64) - 680.0 * scale) / 2.0;
        let y = (sz.height as f64) * 0.16;
        launcher_win.set_outer_position(PhysicalPosition::new(x as i32, y as i32));
    }

    // File index (NTFS MFT) for instant file search + live USN watching (needs admin).
    // "Lite mode": config.json {"file_search":false} skips it -> host ~150 MB instead of ~750.
    let file_index: mft::Index = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let file_search_on = std::fs::read_to_string(r"R:\Projects\batcave-dashboard\config.json")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["file_search"].as_bool())
        .unwrap_or(true);
    if file_search_on {
        mft::watch_all(file_index.clone());
    }

    // Browser open-tabs bridge: localhost server the extension pushes tabs to.
    let browser_tabs: tabs::Tabs = Arc::new(Mutex::new(Vec::new()));
    let tab_pending: tabs::Pending = Arc::new(Mutex::new(None));
    {
        let (t, p) = (browser_tabs.clone(), tab_pending.clone());
        std::thread::spawn(move || tabs::serve(t, p));
    }

    // Plugin host: launch external plugins from the plugins/ folder.
    let plugin_host = Arc::new(plugins::Host::load(std::path::Path::new(
        r"R:\Projects\batcave-dashboard\plugins",
    )));
    let plugins_json = plugin_host
        .list()
        .iter()
        .map(|(n, t)| format!(r#"{{"name":"{}","trigger":"{}"}}"#, esc(n), esc(t)))
        .collect::<Vec<_>>()
        .join(",");

    // Currency rates: fetch once in the background; pushed to the launcher on show.
    let currency_json: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    {
        let cj = currency_json.clone();
        std::thread::spawn(move || {
            let rates = net::currency_rates();
            if !rates.is_empty() {
                let json = rates
                    .iter()
                    .map(|(c, r)| format!(r#""{c}":{r}"#))
                    .collect::<Vec<_>>()
                    .join(",");
                if let Ok(mut s) = cj.lock() {
                    *s = json;
                }
            }
        });
    }

    // GitHub repos + notifications: prefetch via `gh` in the background.
    let gh_repos_json: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let gh_notifs_json: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    {
        let (rj, nj) = (gh_repos_json.clone(), gh_notifs_json.clone());
        std::thread::spawn(move || {
            let repos = github::repos()
                .iter()
                .map(|(n, u)| format!(r#"{{"name":"{}","url":"{}"}}"#, esc(n), esc(u)))
                .collect::<Vec<_>>()
                .join(",");
            if let Ok(mut s) = rj.lock() {
                *s = repos;
            }
            let notifs = github::notifications()
                .iter()
                .map(|(t, u)| format!(r#"{{"name":"{}","url":"{}"}}"#, esc(t), esc(u)))
                .collect::<Vec<_>>()
                .join(",");
            if let Ok(mut s) = nj.lock() {
                *s = notifs;
            }
        });
    }

    // Clipboard history: poll the clipboard, keep the last ~40 distinct copies + their source app.
    let clip_history: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let ch = clip_history.clone();
        std::thread::spawn(move || {
            let mut cb = arboard::Clipboard::new().ok();
            let mut last = String::new();
            loop {
                if let Some(c) = cb.as_mut() {
                    if let Ok(txt) = c.get_text() {
                        if !txt.trim().is_empty() && txt != last {
                            last = txt.clone();
                            let src = unsafe {
                                let mut pid = 0u32;
                                GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
                                process_name(pid)
                            };
                            let src = if src.is_empty() || src.eq_ignore_ascii_case("host.exe") {
                                "clipboard".to_string()
                            } else {
                                src
                            };
                            if let Ok(mut h) = ch.lock() {
                                h.retain(|(t, _)| t != &txt);
                                h.insert(0, (txt, src));
                                h.truncate(40);
                            }
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(800));
            }
        });
    }

    let root2 = ui_dir.clone();
    let proxy_l = event_loop.create_proxy();
    // API credentials from tokens.json (Spotify / Notion), moved into the ipc handler.
    let tokens: serde_json::Value = std::fs::read_to_string(r"R:\Projects\batcave-dashboard\tokens.json")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let sp_id = tokens["spotify_id"].as_str().unwrap_or("").to_string();
    let sp_secret = tokens["spotify_secret"].as_str().unwrap_or("").to_string();
    let notion_tok = tokens["notion"].as_str().unwrap_or("").to_string();
    let ch_ipc = clip_history.clone();
    let fi_ipc = file_index.clone();
    let tabs_ipc = browser_tabs.clone();
    let tabp_ipc = tab_pending.clone();
    let ph_ipc = plugin_host.clone();
    let launcher_wv = WebViewBuilder::new_with_web_context(&mut web_ctx)
        .with_transparent(false)
        .with_devtools(true)
        .with_custom_protocol("bat".into(), move |_id, request| {
            let p = request.uri().path().trim_start_matches('/');
            let rel = if p.is_empty() { "launcher.html" } else { p };
            let file = root2.join(rel);
            let body = std::fs::read(&file).unwrap_or_default();
            let mime = match file.extension().and_then(|e| e.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("js") => "text/javascript; charset=utf-8",
                Some("png") => "image/png",
                Some("json") => "application/json",
                _ => "application/octet-stream",
            };
            wry::http::Response::builder()
                .header("Content-Type", mime)
                .header("Access-Control-Allow-Origin", "*")
                .header("Cache-Control", "no-store, must-revalidate")
                .body(Cow::from(body))
                .unwrap()
        })
        .with_ipc_handler(move |req| {
            let body = req.body().to_string();
            if let Some(path) = body.strip_prefix("launch:") {
                let path = path.to_string();
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start", "", &path])
                        .spawn();
                });
                let _ = proxy_l.send_event(UserEvent::HideLauncher);
            } else if body == "launcher:hide" {
                let _ = proxy_l.send_event(UserEvent::HideLauncher);
            } else if let Some(h) = body.strip_prefix("resize:") {
                if let Ok(hv) = h.trim().parse::<f64>() {
                    let _ = proxy_l.send_event(UserEvent::ResizeLauncher(hv));
                }
            } else if let Some(msg) = body.strip_prefix("notif:") {
                let _ = proxy_l.send_event(UserEvent::Notif(msg.to_string()));
                let _ = proxy_l.send_event(UserEvent::HideLauncher);
            } else if let Some(url) = body.strip_prefix("open:") {
                let u = url.to_string();
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("cmd").args(["/C", "start", "", &u]).spawn();
                });
            } else if let Some(cmd) = body.strip_prefix("sys:") {
                let c = cmd.to_string();
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("cmd").args(["/C", &c]).spawn();
                });
            } else if let Some(txt) = body.strip_prefix("copy:") {
                let t = txt.to_string();
                std::thread::spawn(move || {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(t);
                    }
                });
            } else if let Some(cmd) = body.strip_prefix("media:") {
                let c = cmd.to_string();
                std::thread::spawn(move || media_command(&c));
            } else if let Some(v) = body.strip_prefix("vol:") {
                let v = v.to_string();
                std::thread::spawn(move || match v.as_str() {
                    "mute" => set_mute(true),
                    "unmute" => set_mute(false),
                    _ => {
                        if let Ok(n) = v.parse::<f32>() {
                            set_master_volume(n);
                        }
                    }
                });
            } else if let Some(b) = body.strip_prefix("bright:") {
                if let Ok(n) = b.parse::<u32>() {
                    std::thread::spawn(move || set_brightness(n));
                }
            } else if let Some(rest) = body.strip_prefix("appvol:") {
                if let Some((app, pct)) = rest.rsplit_once(':') {
                    let app = app.to_string();
                    if let Ok(n) = pct.parse::<f32>() {
                        std::thread::spawn(move || set_app_volume(&app, n));
                    }
                }
            } else if let Some(rest) = body.strip_prefix("shell:") {
                if let Some((mode, cmd)) = rest.split_once(':') {
                    let mode = mode.to_string();
                    let cmd = cmd.to_string();
                    std::thread::spawn(move || run_shell(&mode, &cmd));
                }
            } else if body == "clipreset" {
                if let Ok(mut h) = ch_ipc.lock() {
                    h.clear();
                }
            } else if body == "togglefs" {
                let path = r"R:\Projects\batcave-dashboard\config.json";
                let cur = std::fs::read_to_string(path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                    .and_then(|v| v["file_search"].as_bool())
                    .unwrap_or(true);
                let _ = std::fs::write(path, format!("{{\"file_search\":{}}}", !cur));
                // restart the host (via the scheduled task) to apply the mode change.
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "timeout /t 2 >nul & schtasks /run /tn BatcaveHost"])
                    .spawn();
                std::process::exit(0);
            } else if let Some(idx) = body.strip_prefix("clip:") {
                if let Ok(i) = idx.parse::<usize>() {
                    if let Some(txt) = ch_ipc.lock().ok().and_then(|h| h.get(i).map(|(t, _)| t.clone())) {
                        std::thread::spawn(move || {
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                let _ = cb.set_text(txt);
                            }
                        });
                    }
                }
            } else if let Some(q) = body.strip_prefix("sp:") {
                let (id, sec, p, q) = (sp_id.clone(), sp_secret.clone(), proxy_l.clone(), q.to_string());
                std::thread::spawn(move || {
                    let arr = spotify::search(&id, &sec, &q)
                        .iter()
                        .map(|(n, a, u)| format!(r#"{{"name":"{}","artist":"{}","url":"{}"}}"#, esc(n), esc(a), esc(u)))
                        .collect::<Vec<_>>()
                        .join(",");
                    let qj = serde_json::to_string(&q).unwrap_or_default();
                    let _ = p.send_event(UserEvent::EvalLauncher(format!(
                        "window.pushSpotify && window.pushSpotify({qj}, [{arr}]);"
                    )));
                });
            } else if let Some(q) = body.strip_prefix("file:") {
                let (fi, p, q) = (fi_ipc.clone(), proxy_l.clone(), q.to_string());
                std::thread::spawn(move || {
                    let ql = q.to_lowercase();
                    let arr = match fi.lock() {
                        Ok(idx) => {
                            let mut hits: Vec<(&String, u8)> = Vec::new();
                            for path in idx.values() {
                                let fname = path.rsplit('\\').next().unwrap_or(path);
                                if let Some(pos) = fname.to_ascii_lowercase().find(&ql) {
                                    hits.push((path, if pos == 0 { 2 } else { 1 }));
                                }
                            }
                            hits.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.len().cmp(&b.0.len())));
                            hits.truncate(500);
                            hits.iter()
                                .map(|(p, _)| {
                                    let name = p.rsplit('\\').next().unwrap_or(p);
                                    format!(r#"{{"name":"{}","path":"{}"}}"#, esc(name), esc(p))
                                })
                                .collect::<Vec<_>>()
                                .join(",")
                        }
                        Err(_) => String::new(),
                    };
                    let qj = serde_json::to_string(&q).unwrap_or_default();
                    let _ = p.send_event(UserEvent::EvalLauncher(format!(
                        "window.pushFiles && window.pushFiles({qj}, [{arr}]);"
                    )));
                });
            } else if let Some(q) = body.strip_prefix("proc:") {
                let (p, q) = (proxy_l.clone(), q.to_string());
                std::thread::spawn(move || {
                    let arr = proc::find(&q)
                        .iter()
                        .map(|(n, pid, mem)| {
                            format!(r#"{{"name":"{}","pid":{},"mem":{}}}"#, esc(n), pid, mem)
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let qj = serde_json::to_string(&q).unwrap_or_default();
                    let _ = p.send_event(UserEvent::EvalLauncher(format!(
                        "window.pushProcs && window.pushProcs({qj}, [{arr}]);"
                    )));
                });
            } else if let Some(pid) = body.strip_prefix("killpid:") {
                if let Ok(pid) = pid.parse::<u32>() {
                    std::thread::spawn(move || {
                        proc::kill(pid);
                    });
                }
            } else if let Some(q) = body.strip_prefix("tabsearch:") {
                let ql = q.to_lowercase();
                let arr = tabs_ipc
                    .lock()
                    .map(|list| {
                        list.iter()
                            .filter(|(t, u, _, _)| {
                                t.to_lowercase().contains(&ql) || u.to_lowercase().contains(&ql)
                            })
                            .take(8)
                            .map(|(t, u, id, win)| {
                                format!(r#"{{"title":"{}","url":"{}","id":{},"win":{}}}"#, esc(t), esc(u), id, win)
                            })
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_default();
                let qj = serde_json::to_string(q).unwrap_or_default();
                let _ = proxy_l.send_event(UserEvent::EvalLauncher(format!(
                    "window.pushTabs && window.pushTabs({qj}, [{arr}]);"
                )));
            } else if let Some(rest) = body.strip_prefix("tabactivate:") {
                if let Some((tid, wid)) = rest.split_once(':') {
                    if let (Ok(tid), Ok(wid)) = (tid.parse::<i64>(), wid.parse::<i64>()) {
                        if let Ok(mut g) = tabp_ipc.lock() {
                            *g = Some((tid, wid));
                        }
                    }
                }
            } else if let Some(q) = body.strip_prefix("plugin:") {
                let (ph, p, q) = (ph_ipc.clone(), proxy_l.clone(), q.to_string());
                std::thread::spawn(move || {
                    let arr = ph
                        .query(&q)
                        .iter()
                        .map(|(t, s, a)| {
                            format!(r#"{{"title":"{}","subtitle":"{}","action":"{}"}}"#, esc(t), esc(s), esc(a))
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let qj = serde_json::to_string(&q).unwrap_or_default();
                    let _ = p.send_event(UserEvent::EvalLauncher(format!(
                        "window.pushPlugins && window.pushPlugins({qj}, [{arr}]);"
                    )));
                });
            } else if let Some(q) = body.strip_prefix("ns:") {
                let (tok, p, q) = (notion_tok.clone(), proxy_l.clone(), q.to_string());
                std::thread::spawn(move || {
                    let arr = notion::search(&tok, &q)
                        .iter()
                        .map(|(t, u)| format!(r#"{{"title":"{}","url":"{}"}}"#, esc(t), esc(u)))
                        .collect::<Vec<_>>()
                        .join(",");
                    let qj = serde_json::to_string(&q).unwrap_or_default();
                    let _ = p.send_event(UserEvent::EvalLauncher(format!(
                        "window.pushNotion && window.pushNotion({qj}, [{arr}]);"
                    )));
                });
            }
        })
        .with_url("bat://localhost/launcher.html")
        .build(&launcher_win)?;

    // Enumerate installed apps once; re-pushed to the launcher on every show (below).
    let apps = enum_apps();
    let apps_json = apps
        .iter()
        .map(|(n, p)| format!(r#"{{"name":"{}","path":"{}"}}"#, esc(n), esc(p)))
        .collect::<Vec<_>>()
        .join(",");
    let _ = launcher_wv.evaluate_script(&format!("window.pushApps && window.pushApps([{apps_json}]);"));

    // Extract each app's real shell icon off-thread; push the path->image map when ready.
    {
        let paths: Vec<String> = apps.iter().map(|(_, p)| p.clone()).collect();
        let px = event_loop.create_proxy();
        std::thread::spawn(move || {
            let mut parts = Vec::new();
            for p in paths {
                if let Some(data) = icons::extract(&p) {
                    parts.push(format!(r#""{}":"{}""#, esc(&p), data));
                }
            }
            let _ = px.send_event(UserEvent::EvalLauncher(format!(
                "window.pushAppIcons && window.pushAppIcons({{{}}});",
                parts.join(",")
            )));
        });
    }

    let steam_json = {
        let games = enum_steam();
        games
            .iter()
            .map(|(a, n)| format!(r#"{{"appid":"{}","name":"{}"}}"#, esc(a), esc(n)))
            .collect::<Vec<_>>()
            .join(",")
    };
    let _ = launcher_wv
        .evaluate_script(&format!("window.pushSteam && window.pushSteam([{steam_json}]);"));

    let obsidian_json = {
        let notes = enum_obsidian();
        notes
            .iter()
            .map(|(n, p)| format!(r#"{{"name":"{}","path":"{}"}}"#, esc(n), esc(p)))
            .collect::<Vec<_>>()
            .join(",")
    };
    let _ = launcher_wv.evaluate_script(&format!(
        "window.pushObsidian && window.pushObsidian([{obsidian_json}]);"
    ));

    let bookmarks_json = browser::enum_bookmarks()
        .iter()
        .map(|(n, u)| format!(r#"{{"name":"{}","url":"{}"}}"#, esc(n), esc(u)))
        .collect::<Vec<_>>()
        .join(",");
    let history_json = browser::enum_history()
        .iter()
        .map(|(n, u)| format!(r#"{{"name":"{}","url":"{}"}}"#, esc(n), esc(u)))
        .collect::<Vec<_>>()
        .join(",");

    // Alt+Space opens the launcher. Kept alive for the process lifetime via `_hk_manager`.
    let _hk_manager = GlobalHotKeyManager::new().ok();
    if let Some(m) = &_hk_manager {
        let _ = m.register(HotKey::new(Some(Modifiers::ALT), Code::Space));
    }
    let proxy_hk = event_loop.create_proxy();
    std::thread::spawn(move || {
        let rx = GlobalHotKeyEvent::receiver();
        while let Ok(ev) = rx.recv() {
            if ev.state == HotKeyState::Pressed {
                let _ = proxy_hk.send_event(UserEvent::ToggleLauncher);
            }
        }
    });

    // ---- Toast (surface #3): small OPAQUE always-on-top banner, shows over apps ----
    let toast_win = WindowBuilder::new()
        .with_decorations(false)
        .with_transparent(false)
        .with_visible(false)
        .with_always_on_top(true)
        .with_inner_size(LogicalSize::new(500.0, 104.0))
        .build(&event_loop)
        .expect("toast window failed");
    {
        let scale = toast_win.scale_factor();
        let x = ((sz.width as f64) - 500.0 * scale) / 2.0;
        let y = (sz.height as f64) * 0.035;
        toast_win.set_outer_position(PhysicalPosition::new(x as i32, y as i32));
    }
    let root_t = ui_dir.clone();
    let toast_wv = WebViewBuilder::new_with_web_context(&mut web_ctx)
        .with_transparent(false)
        .with_custom_protocol("bat".into(), move |_id, request| {
            let p = request.uri().path().trim_start_matches('/');
            let rel = if p.is_empty() { "toast.html" } else { p };
            let file = root_t.join(rel);
            let body = std::fs::read(&file).unwrap_or_default();
            let mime = match file.extension().and_then(|e| e.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("js") => "text/javascript; charset=utf-8",
                Some("png") => "image/png",
                Some("json") => "application/json",
                _ => "application/octet-stream",
            };
            wry::http::Response::builder()
                .header("Content-Type", mime)
                .header("Access-Control-Allow-Origin", "*")
                .header("Cache-Control", "no-store, must-revalidate")
                .body(Cow::from(body))
                .unwrap()
        })
        .with_url("bat://localhost/toast.html")
        .build(&toast_win)?;


    // ---- Interactivity: forward desktop mouse events into the webview ----
    SELFW.store(hwnd.0 as isize, Ordering::Relaxed);
    unsafe {
        if let Ok(hmod) = GetModuleHandleW(None) {
            let _ = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), Some(HINSTANCE(hmod.0)), 0);
        }
    }
    // WebView2 spawns its Chromium window a bit later, so wait for it then forward clicks.
    let self_isize = hwnd.0 as isize;
    std::thread::spawn(move || {
        for _ in 0..40 {
            std::thread::sleep(Duration::from_millis(250));
            let mut wv: HWND = HWND(std::ptr::null_mut());
            unsafe {
                let parent = HWND(self_isize as *mut std::ffi::c_void);
                let _ = EnumChildWindows(
                    Some(parent),
                    Some(find_wv),
                    LPARAM(&mut wv as *mut HWND as isize),
                );
            }
            if !wv.0.is_null() {
                TARGET.store(wv.0 as isize, Ordering::Relaxed);
                break;
            }
        }
    });

    let mut launcher_visible = false;
    let mut launcher_shown_at: Option<std::time::Instant> = None;
    let mut prev_fg: isize = 0; // foreground window captured when the launcher opens
    let mut pushed_static = false; // apps/steam/obsidian only need pushing once
    event_loop.run(move |event, _, cf| {
        *cf = ControlFlow::Wait;
        match event {
            Event::UserEvent(UserEvent::Stats(json)) => {
                let _ = webview
                    .evaluate_script(&format!("window.pushStats && window.pushStats({json});"));
            }
            Event::UserEvent(UserEvent::NowPlaying(json)) => {
                let _ = webview.evaluate_script(&format!(
                    "window.pushNowPlaying && window.pushNowPlaying({json});"
                ));
            }
            Event::UserEvent(UserEvent::Viz(json)) => {
                let _ = webview.evaluate_script(&format!("window.pushViz && window.pushViz([{json}]);"));
            }
            Event::UserEvent(UserEvent::Reload) => {
                let _ = webview.evaluate_script("location.reload()");
                let _ = launcher_wv.evaluate_script("location.reload()");
                let _ = toast_wv.evaluate_script("location.reload()");
                pushed_static = false; // re push apps/steam/etc. on next launcher open
            }
            Event::UserEvent(UserEvent::ToggleLauncher) => {
                let fg = unsafe { GetForegroundWindow() }.0 as isize;
                launcher_visible = !launcher_visible;
                launcher_win.set_visible(launcher_visible);
                if launcher_visible {
                    prev_fg = fg; // remember what was focused before we stole it
                    launcher_win.set_focus();
                    launcher_shown_at = Some(std::time::Instant::now());
                    if !pushed_static {
                        pushed_static = true;
                        let _ = launcher_wv.evaluate_script(&format!(
                            "window.pushApps && window.pushApps([{apps_json}]);"
                        ));
                        let _ = launcher_wv.evaluate_script(&format!(
                            "window.pushSteam && window.pushSteam([{steam_json}]);"
                        ));
                        let _ = launcher_wv.evaluate_script(&format!(
                            "window.pushObsidian && window.pushObsidian([{obsidian_json}]);"
                        ));
                        let _ = launcher_wv.evaluate_script(&format!(
                            "window.pushBookmarks && window.pushBookmarks([{bookmarks_json}]);"
                        ));
                        let _ = launcher_wv.evaluate_script(&format!(
                            "window.pushHistory && window.pushHistory([{history_json}]);"
                        ));
                        let _ = launcher_wv.evaluate_script(&format!(
                            "window.pushPluginList && window.pushPluginList([{plugins_json}]);"
                        ));
                        let _ = launcher_wv.evaluate_script(&format!(
                            "window.pushFsState && window.pushFsState({file_search_on});"
                        ));
                    }
                    let clip_json = clip_history
                        .lock()
                        .map(|h| {
                            h.iter()
                                .map(|(t, s)| format!(r#"{{"t":"{}","s":"{}"}}"#, esc(t), esc(s)))
                                .collect::<Vec<_>>()
                                .join(",")
                        })
                        .unwrap_or_default();
                    let _ = launcher_wv.evaluate_script(&format!(
                        "window.pushClips && window.pushClips([{clip_json}]);"
                    ));
                    let rates = currency_json.lock().map(|s| s.clone()).unwrap_or_default();
                    let _ = launcher_wv.evaluate_script(&format!(
                        "window.pushRates && window.pushRates({{{rates}}});"
                    ));
                    let ghr = gh_repos_json.lock().map(|s| s.clone()).unwrap_or_default();
                    let _ = launcher_wv
                        .evaluate_script(&format!("window.pushRepos && window.pushRepos([{ghr}]);"));
                    let ghn = gh_notifs_json.lock().map(|s| s.clone()).unwrap_or_default();
                    let _ = launcher_wv.evaluate_script(&format!(
                        "window.pushNotifs && window.pushNotifs([{ghn}]);"
                    ));
                    let _ = launcher_wv
                        .evaluate_script("window.onLauncherShow && window.onLauncherShow();");
                }
            }
            Event::UserEvent(UserEvent::HideLauncher) => {
                launcher_visible = false;
                launcher_win.set_visible(false);
            }
            Event::UserEvent(UserEvent::ResizeLauncher(h)) => {
                launcher_win.set_inner_size(LogicalSize::new(680.0, h.max(60.0)));
            }
            Event::UserEvent(UserEvent::EvalLauncher(js)) => {
                let _ = launcher_wv.evaluate_script(&js);
            }
            Event::UserEvent(UserEvent::Notif(msg)) => {
                // desktop foreground -> big cinematic; an app -> toast over it
                let mode = if foreground_is_desktop(prev_fg) { "desktop" } else { "app" };
                let m = esc(&msg);
                let _ = webview.evaluate_script(&format!(
                    "window.pushNotif && window.pushNotif(\"{m}\",\"{mode}\");"
                ));
            }
            Event::UserEvent(UserEvent::ShowToast(msg)) => {
                toast_win.set_visible(true);
                let m = esc(&msg);
                let _ = toast_wv
                    .evaluate_script(&format!("window.playToast && window.playToast(\"{m}\");"));
            }
            Event::UserEvent(UserEvent::HideToast) => {
                toast_win.set_visible(false);
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Focused(false),
                ..
            } if window_id == launcher_id && launcher_visible => {
                let fresh = launcher_shown_at
                    .map(|t| t.elapsed().as_millis() < 600)
                    .unwrap_or(false);
                if !fresh {
                    launcher_visible = false;
                    launcher_win.set_visible(false);
                }
            }
            _ => {}
        }
    });
}
