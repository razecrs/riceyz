//! System media transport controls (SMTC): now-playing + play/pause/skip.
use std::time::Duration;

use windows::core::RuntimeType;
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as Smtc;
use windows_future::{AsyncStatus, IAsyncOperation};

/// Block on a WinRT async operation without an async runtime.
fn block<T: RuntimeType>(op: IAsyncOperation<T>) -> Option<T> {
    for _ in 0..400 {
        match op.Status() {
            Ok(AsyncStatus::Completed) => return op.GetResults().ok(),
            Ok(AsyncStatus::Started) => std::thread::sleep(Duration::from_millis(5)),
            _ => return None,
        }
    }
    None
}

/// Current media session -> (title, artist, is_playing).
pub fn now_playing() -> Option<(String, String, bool)> {
    let mgr = block(Smtc::RequestAsync().ok()?)?;
    let session = mgr.GetCurrentSession().ok()?;
    let props = block(session.TryGetMediaPropertiesAsync().ok()?)?;
    let title = props.Title().map(|h| h.to_string()).unwrap_or_default();
    let artist = props.Artist().map(|h| h.to_string()).unwrap_or_default();
    let playing = session
        .GetPlaybackInfo()
        .ok()
        .and_then(|pi| pi.PlaybackStatus().ok())
        .map(|s| s.0 == 4) // 4 = Playing
        .unwrap_or(false);
    Some((title, artist, playing))
}

pub fn media_command(cmd: &str) {
    if let Some(session) = Smtc::RequestAsync()
        .ok()
        .and_then(block)
        .and_then(|m| m.GetCurrentSession().ok())
    {
        match cmd {
            "playpause" => {
                let _ = session.TryTogglePlayPauseAsync();
            }
            "next" => {
                let _ = session.TrySkipNextAsync();
            }
            "prev" => {
                let _ = session.TrySkipPreviousAsync();
            }
            _ => {}
        }
    }
}
