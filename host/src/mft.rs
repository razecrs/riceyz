//! Native instant file search: full MFT scan + live USN-journal watching.
//! Requires elevation (raw volume access). Index stays current as files change.
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{
    FSCTL_ENUM_USN_DATA, FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL,
};
use windows::Win32::System::IO::DeviceIoControl;

/// (drive<<64 | frn) -> full path.
pub type Index = Arc<Mutex<HashMap<u128, String>>>;

const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const USN_REASON_FILE_DELETE: u32 = 0x0000_0200;

type Frn = u64;
type FrnMap = HashMap<Frn, (String, Frn, bool)>; // frn -> (name, parent, is_dir)

fn ckey(drive: char, frn: Frn) -> u128 {
    ((drive as u128) << 64) | frn as u128
}

fn open_volume(drive: char) -> Option<HANDLE> {
    unsafe {
        let vol: Vec<u16> = format!(r"\\.\{drive}:").encode_utf16().chain(Some(0)).collect();
        CreateFileW(
            PCWSTR(vol.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
        .ok()
    }
}

fn build_path(frn: Frn, map: &FrnMap, drive: char, cache: &mut HashMap<Frn, String>) -> String {
    if let Some(p) = cache.get(&frn) {
        return p.clone();
    }
    let path = match map.get(&frn) {
        Some((name, parent, _)) => {
            let base = if *parent == frn {
                format!("{drive}:")
            } else {
                build_path(*parent, map, drive, cache)
            };
            format!(r"{base}\{name}")
        }
        None => format!("{drive}:"),
    };
    cache.insert(frn, path.clone());
    path
}

/// Parse a USN_RECORD_V2 -> (record_len, frn, parent, name, is_dir, reason).
fn parse_record(rec: &[u8]) -> Option<(usize, Frn, Frn, String, bool, u32)> {
    if rec.len() < 60 {
        return None;
    }
    let len = u32::from_le_bytes(rec[0..4].try_into().ok()?) as usize;
    if len == 0 || len > rec.len() {
        return None;
    }
    let frn = u64::from_le_bytes(rec[8..16].try_into().ok()?);
    let parent = u64::from_le_bytes(rec[16..24].try_into().ok()?);
    let reason = u32::from_le_bytes(rec[40..44].try_into().ok()?);
    let attrs = u32::from_le_bytes(rec[52..56].try_into().ok()?);
    let nlen = u16::from_le_bytes(rec[56..58].try_into().ok()?) as usize;
    let noff = u16::from_le_bytes(rec[58..60].try_into().ok()?) as usize;
    let is_dir = attrs & FILE_ATTRIBUTE_DIRECTORY != 0;
    if noff + nlen > len || noff + nlen > rec.len() {
        return Some((len, frn, parent, String::new(), is_dir, reason));
    }
    let name: Vec<u16> = rec[noff..noff + nlen]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    Some((len, frn, parent, String::from_utf16_lossy(&name), is_dir, reason))
}

/// Full MFT enumeration -> populate the FRN map.
fn enum_mft(handle: HANDLE, map: &mut FrnMap) {
    unsafe {
        let mut med = [0u8; 24];
        med[16..24].copy_from_slice(&i64::MAX.to_le_bytes());
        let mut buf = vec![0u8; 1 << 16];
        loop {
            let mut ret = 0u32;
            let ok = DeviceIoControl(
                handle,
                FSCTL_ENUM_USN_DATA,
                Some(med.as_ptr() as *const c_void),
                24,
                Some(buf.as_mut_ptr() as *mut c_void),
                buf.len() as u32,
                Some(&mut ret),
                None,
            );
            if ok.is_err() || ret <= 8 {
                break;
            }
            let next = u64::from_le_bytes(buf[0..8].try_into().unwrap());
            let mut off = 8usize;
            while off + 60 <= ret as usize {
                match parse_record(&buf[off..ret as usize]) {
                    Some((len, frn, parent, name, is_dir, _)) => {
                        if !name.is_empty() {
                            map.insert(frn, (name, parent, is_dir));
                        }
                        off += len;
                    }
                    None => break,
                }
            }
            med[0..8].copy_from_slice(&next.to_le_bytes());
        }
    }
}

/// Query the USN journal -> (journal id, next usn).
fn query_journal(handle: HANDLE) -> Option<(u64, i64)> {
    unsafe {
        let mut buf = [0u8; 80];
        let mut ret = 0u32;
        let ok = DeviceIoControl(
            handle,
            FSCTL_QUERY_USN_JOURNAL,
            None,
            0,
            Some(buf.as_mut_ptr() as *mut c_void),
            buf.len() as u32,
            Some(&mut ret),
            None,
        );
        if ok.is_err() {
            return None;
        }
        let jid = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        let next = i64::from_le_bytes(buf[16..24].try_into().unwrap());
        Some((jid, next))
    }
}

fn run_drive(drive: char, index: Index) {
    let handle = match open_volume(drive) {
        Some(h) => h,
        None => return, // not elevated / not NTFS
    };
    let mut map = FrnMap::new();
    enum_mft(handle, &mut map);

    // Initial full population.
    {
        let mut cache = HashMap::new();
        let frns: Vec<Frn> = map.keys().copied().collect();
        if let Ok(mut idx) = index.lock() {
            for frn in &frns {
                idx.insert(ckey(drive, *frn), build_path(*frn, &map, drive, &mut cache));
            }
        }
    }

    // Live watch via the USN journal.
    let (jid, mut next) = match query_journal(handle) {
        Some(x) => x,
        None => {
            let _ = unsafe { CloseHandle(handle) };
            return;
        }
    };
    let mut rjd = [0u8; 40];
    rjd[8..12].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // ReasonMask = all
    rjd[32..40].copy_from_slice(&jid.to_le_bytes()); // UsnJournalID
    let mut buf = vec![0u8; 1 << 16];
    loop {
        rjd[0..8].copy_from_slice(&next.to_le_bytes()); // StartUsn
        let mut ret = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_READ_USN_JOURNAL,
                Some(rjd.as_ptr() as *const c_void),
                40,
                Some(buf.as_mut_ptr() as *mut c_void),
                buf.len() as u32,
                Some(&mut ret),
                None,
            )
        };
        if ok.is_err() || ret < 8 {
            std::thread::sleep(Duration::from_millis(1500));
            continue;
        }
        next = i64::from_le_bytes(buf[0..8].try_into().unwrap());
        let mut off = 8usize;
        let mut changed = false;
        let mut cache = HashMap::new();
        while off + 60 <= ret as usize {
            match parse_record(&buf[off..ret as usize]) {
                Some((len, frn, parent, name, is_dir, reason)) => {
                    if !name.is_empty() {
                        if reason & USN_REASON_FILE_DELETE != 0 {
                            map.remove(&frn);
                            if let Ok(mut idx) = index.lock() {
                                idx.remove(&ckey(drive, frn));
                            }
                        } else {
                            map.insert(frn, (name, parent, is_dir));
                            let path = build_path(frn, &map, drive, &mut cache);
                            if let Ok(mut idx) = index.lock() {
                                idx.insert(ckey(drive, frn), path);
                            }
                        }
                        changed = true;
                    }
                    off += len;
                }
                None => break,
            }
        }
        if !changed {
            std::thread::sleep(Duration::from_millis(700)); // journal drained -> poll
        }
    }
}

/// Build + live-watch the file index across all fixed NTFS volumes.
pub fn watch_all(index: Index) {
    for d in 'C'..='Z' {
        if std::path::Path::new(&format!("{d}:\\")).is_dir() {
            let idx = index.clone();
            std::thread::spawn(move || run_drive(d, idx));
        }
    }
}
