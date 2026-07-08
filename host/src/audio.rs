//! Core Audio: master + per-app volume, mute; PID -> exe name.
use windows::core::{Interface, PWSTR};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2, ISimpleAudioVolume,
    IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

unsafe fn endpoint_volume() -> Option<IAudioEndpointVolume> {
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
    let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
    device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()
}

pub fn set_master_volume(pct: f32) {
    unsafe {
        if let Some(vol) = endpoint_volume() {
            let level = (pct / 100.0).clamp(0.0, 1.0);
            let _ = vol.SetMasterVolumeLevelScalar(level, std::ptr::null());
        }
    }
}

pub fn set_mute(mute: bool) {
    unsafe {
        if let Some(vol) = endpoint_volume() {
            let _ = vol.SetMute(mute, std::ptr::null());
        }
    }
}

/// Resolve a PID to its exe basename (e.g. "chrome.exe").
pub unsafe fn process_name(pid: u32) -> String {
    if pid == 0 {
        return String::new();
    }
    let h = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
        Ok(h) => h,
        Err(_) => return String::new(),
    };
    let mut buf = [0u16; 260];
    let mut len = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
    let _ = CloseHandle(h);
    if ok {
        String::from_utf16_lossy(&buf[..len as usize])
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    }
}

/// Set the volume of every audio session whose process matches `app`.
pub fn set_app_volume(app: &str, pct: f32) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
            Ok(e) => e,
            Err(_) => return,
        };
        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(_) => return,
        };
        let manager: IAudioSessionManager2 = match device.Activate(CLSCTX_ALL, None) {
            Ok(m) => m,
            Err(_) => return,
        };
        let sessions = match manager.GetSessionEnumerator() {
            Ok(s) => s,
            Err(_) => return,
        };
        let target = app.to_lowercase();
        let target = target.trim_end_matches(".exe");
        let level = (pct / 100.0).clamp(0.0, 1.0);
        for i in 0..sessions.GetCount().unwrap_or(0) {
            if let Ok(ctrl) = sessions.GetSession(i) {
                let pid = ctrl
                    .cast::<IAudioSessionControl2>()
                    .ok()
                    .and_then(|c2| c2.GetProcessId().ok())
                    .unwrap_or(0);
                let name = process_name(pid).to_lowercase();
                let base = name.trim_end_matches(".exe");
                if !base.is_empty() && (base == target || base.contains(target)) {
                    if let Ok(simple) = ctrl.cast::<ISimpleAudioVolume>() {
                        let _ = simple.SetMasterVolume(level, std::ptr::null());
                    }
                }
            }
        }
    }
}
