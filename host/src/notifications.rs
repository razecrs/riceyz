//! Surface real Windows toast notifications through our own batarang.
use std::collections::HashSet;

use windows::core::RuntimeType;
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds, UserNotification};
use windows_future::{AsyncStatus, IAsyncOperation};

fn block<T: RuntimeType>(op: IAsyncOperation<T>) -> Option<T> {
    for _ in 0..400 {
        match op.Status() {
            Ok(AsyncStatus::Completed) => return op.GetResults().ok(),
            Ok(AsyncStatus::Started) => std::thread::sleep(std::time::Duration::from_millis(10)),
            _ => return None,
        }
    }
    None
}

/// Toggle native Windows toast *banners*. When off, notifications still land in the
/// Action Center (so our listener + batarang still fire), it just hides the OS popup.
/// Reversible: pass `true` to restore.
pub fn set_native_toasts(enabled: bool) {
    let val = if enabled { "1" } else { "0" };
    let _ = std::process::Command::new("reg")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings",
            "/v",
            "NOC_GLOBAL_SETTING_TOASTS_ENABLED",
            "/t",
            "REG_DWORD",
            "/d",
            val,
            "/f",
        ])
        .spawn();
}

/// Ask for notification-listener access (prompts once). true if allowed.
pub fn request_access() -> bool {
    UserNotificationListener::Current()
        .ok()
        .and_then(|l| l.RequestAccessAsync().ok())
        .and_then(block)
        .map(|s| s == UserNotificationListenerAccessStatus::Allowed)
        .unwrap_or(false)
}

/// Poll live toast notifications; return (app, text) for ones not already in `seen`.
pub fn poll(seen: &mut HashSet<u32>) -> Vec<(String, String)> {
    let listener = match UserNotificationListener::Current() {
        Ok(l) => l,
        Err(_) => return Vec::new(),
    };
    let notifs = match listener
        .GetNotificationsAsync(NotificationKinds::Toast)
        .ok()
        .and_then(block)
    {
        Some(n) => n,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    let mut current = HashSet::new();
    for i in 0..notifs.Size().unwrap_or(0) {
        let un = match notifs.GetAt(i) {
            Ok(u) => u,
            Err(_) => continue,
        };
        let id = un.Id().unwrap_or(0);
        current.insert(id);
        if seen.contains(&id) {
            continue;
        }
        let app = un
            .AppInfo()
            .ok()
            .and_then(|ai| ai.DisplayInfo().ok())
            .and_then(|di| di.DisplayName().ok())
            .map(|h| h.to_string())
            .unwrap_or_default();
        let text = notif_text(&un);
        if !app.is_empty() || !text.is_empty() {
            out.push((app, text));
        }
    }
    // Bound `seen` to what's still live, so a re-shown notification fires again.
    seen.retain(|id| current.contains(id));
    seen.extend(current);
    out
}

fn notif_text(un: &UserNotification) -> String {
    let binding = un
        .Notification()
        .ok()
        .and_then(|n| n.Visual().ok())
        .and_then(|v| {
            let generic = KnownNotificationBindings::ToastGeneric().ok()?;
            v.GetBinding(&generic).ok()
        });
    let elems = match binding.and_then(|b| b.GetTextElements().ok()) {
        Some(e) => e,
        None => return String::new(),
    };
    let mut parts = Vec::new();
    for i in 0..elems.Size().unwrap_or(0) {
        if let Ok(t) = elems.GetAt(i) {
            if let Ok(s) = t.Text() {
                let s = s.to_string();
                if !s.is_empty() {
                    parts.push(s);
                }
            }
        }
    }
    parts.join(" · ")
}
