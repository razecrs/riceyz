//! Process listing + termination for the launcher's `kill` command.
use sysinfo::{Pid, System};

/// Running processes whose name contains `query` -> (name, pid, mem_bytes), biggest first.
pub fn find(query: &str) -> Vec<(String, u32, u64)> {
    let q = query.to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let sys = System::new_all();
    let mut out: Vec<(String, u32, u64)> = Vec::new();
    for (pid, p) in sys.processes() {
        let name = p.name().to_string_lossy().to_string();
        if name.to_lowercase().contains(&q) {
            out.push((name, pid.as_u32(), p.memory()));
        }
    }
    out.sort_by(|a, b| b.2.cmp(&a.2));
    out.truncate(20);
    out
}

/// Kill a process by pid. Returns true on success.
pub fn kill(pid: u32) -> bool {
    let sys = System::new_all();
    sys.process(Pid::from_u32(pid)).map(|p| p.kill()).unwrap_or(false)
}
