use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn is_root() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::getuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[derive(Serialize, Clone)]
pub struct SystemInfo {
    pub distro: String,
    pub desktop: String,
    pub is_gnome: bool,
    pub is_kde: bool,
    pub is_root: bool,
}

#[derive(Serialize, Clone, Default)]
pub struct ScanResult {
    pub remove_snap: bool,
    pub install_flatpak: bool,
    pub firefox_ppa: bool,
    pub telemetry_off: bool,
    pub apport_off: bool,
    pub motd_off: bool,
    pub swappiness_tuned: bool,
    pub shutdown_fast: bool,
    pub ssd_trim: bool,
    pub tracker_off: bool,
    pub baloo_off: bool,
    pub bloat_removed: bool,
}

pub fn which(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn detect_system() -> SystemInfo {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .unwrap_or_default();
    let upper = desktop.to_uppercase();
    let is_gnome = upper.contains("GNOME") || upper.contains("UNITY") || upper.contains("BUDGIE");
    let is_kde = upper.contains("KDE") || upper.contains("PLASMA");

    let distro = fs::read_to_string("/etc/os-release")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("PRETTY_NAME="))
        .map(|l| l.replace("PRETTY_NAME=", "").trim_matches('"').to_string())
        .unwrap_or_else(|| "Linux".into());

    SystemInfo {
        distro,
        desktop,
        is_gnome,
        is_kde,
        is_root: is_root(),
    }
}

pub fn scan_system() -> ScanResult {
    let mut r = ScanResult::default();
    let snap = which("snap");
    r.remove_snap = !snap || Path::new("/etc/apt/preferences.d/nosnap.pref").exists();
    r.install_flatpak = which("flatpak");
    r.firefox_ppa = Path::new("/etc/apt/sources.list.d/mozilla.list").exists()
        || Path::new("/etc/apt/preferences.d/mozilla").exists();
    r.telemetry_off = !which("ubuntu-report");
    r.apport_off = fs::read_to_string("/etc/default/apport")
        .map(|s| s.contains("enabled=0"))
        .unwrap_or(!Path::new("/usr/bin/apport-cli").exists());
    r.motd_off = !Path::new("/etc/update-motd.d/50-motd-news").exists();
    r.swappiness_tuned = fs::read_to_string("/proc/sys/vm/swappiness")
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
        .map(|v| v <= 10)
        .unwrap_or(false);
    r.shutdown_fast = fs::read_to_string("/etc/systemd/system.conf")
        .map(|s| s.contains("DefaultTimeoutStopSec=10s"))
        .unwrap_or(false);
    r.ssd_trim = Command::new("systemctl")
        .args(["is-enabled", "fstrim.timer"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "enabled")
        .unwrap_or(false);
    r.tracker_off = Command::new("systemctl")
        .args(["--user", "is-masked", "tracker-miner-fs-3.service"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "masked")
        .unwrap_or(false);
    r.baloo_off = Command::new("balooctl")
        .arg("status")
        .output()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout).to_lowercase();
            s.contains("disabled") || s.contains("not running")
        })
        .unwrap_or(false);
    r.bloat_removed = !Path::new("/usr/games/gnome-mines").exists()
        && !Path::new("/usr/games/kmines").exists();
    r
}

pub fn run_optimizer_script(body: &str) -> Result<String, String> {
    let script = format!(
        "#!/bin/bash\nset -e\nexport DEBIAN_FRONTEND=noninteractive\n{}\n",
        body
    );
    let path = "/tmp/ubuntu_optimizer_exec.sh";
    fs::write(path, &script).map_err(|e| e.to_string())?;
    let _ = Command::new("chmod").args(["+x", path]).status();

    let is_root = is_root();
    let output = if is_root {
        Command::new("bash").arg(path).output()
    } else {
        Command::new("pkexec").arg("bash").arg(path).output()
    };
    let _ = fs::remove_file(path);

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if out.status.success() {
                Ok(stdout)
            } else {
                Err(format!("Failed (exit {:?}):\n{}\n{}", out.status.code(), stdout, stderr))
            }
        }
        Err(e) => Err(format!("Failed to launch: {}", e)),
    }
}
