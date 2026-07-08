//! Detects the machine's hardware once at boot and writes specs.json for the dashboard.
//! Keeps the panel per-machine instead of hard-coding one rig.
use std::process::Command;

/// Query WMI for CPU/GPU/RAM/disks and drop the result into `out_path` as JSON.
/// Runs the whole thing in PowerShell since WMI is the reliable source for this stuff.
pub fn refresh(out_path: &str) {
    let ps = r#"
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$gpus = Get-CimInstance Win32_VideoController | Where-Object { $_.Name -notmatch 'Basic|Remote|Virtual|Meta' }
$gpu = ($gpus | Where-Object { $_.Name -match 'NVIDIA|GeForce|Radeon|AMD|Arc' } | Select-Object -First 1).Name
if (-not $gpu) { $gpu = ($gpus | Select-Object -First 1).Name }
$mem = Get-CimInstance Win32_PhysicalMemory
$disks = Get-CimInstance Win32_DiskDrive | ForEach-Object {
  [pscustomobject]@{ model = $_.Model; sizeGB = [math]::Round($_.Size / 1GB, 0); type = $_.MediaType }
}
[pscustomobject]@{
  cpu      = $cpu.Name.Trim()
  cores    = $cpu.NumberOfCores
  threads  = $cpu.NumberOfLogicalProcessors
  gpu      = $gpu
  ramGB    = [math]::Round(($mem | Measure-Object Capacity -Sum).Sum / 1GB, 0)
  ramSpeed = ($mem | Select-Object -First 1).Speed
  ramType  = "$(($mem | Measure-Object).Count)x sticks"
  disks    = @($disks)
} | ConvertTo-Json -Depth 4
"#;
    if let Ok(o) = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps])
        .output()
    {
        if o.status.success() && !o.stdout.is_empty() {
            let _ = std::fs::write(out_path, o.stdout);
        }
    }
}
