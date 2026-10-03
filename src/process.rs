use std::fs::OpenOptions;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::ports;

const POLL: Duration = Duration::from_millis(100);

pub struct ProcInfo {
    pub started: String,
    pub zombie: bool,
}

/// Start time and state of a pid via ps. None when no such process exists.
pub fn info(pid: u32) -> Option<ProcInfo> {
    let output = Command::new("ps")
        .args(["-o", "stat=", "-o", "lstart=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut words = text.split_whitespace();
    let stat = words.next()?;
    let started = words.collect::<Vec<_>>().join(" ");
    if started.is_empty() {
        return None;
    }
    Some(ProcInfo {
        started,
        zombie: stat.starts_with('Z'),
    })
}

pub fn start_time(pid: u32) -> Option<String> {
    info(pid).map(|i| i.started)
}

/// A pid only counts as the session's process when its start time matches the recorded one, so a reused pid is never mistaken for it.
pub fn is_alive(pid: u32, started: &str) -> bool {
    match info(pid) {
        Some(i) => !i.zombie && (started.is_empty() || i.started == started),
        None => false,
    }
}

fn shell(command: &str, cwd: &Path, port: u16) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(command)
        .current_dir(cwd)
        .env("PORT", port.to_string())
        .stdin(Stdio::null());
    cmd
}

pub fn spawn_detached(command: &str, cwd: &Path, port: u16, log_path: &Path) -> Result<Child> {
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("opening {}", log_path.display()))?;
    let mut cmd = shell(command, cwd, port);
    cmd.stdout(log.try_clone()?).stderr(log);
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    cmd.spawn().with_context(|| format!("spawning: {command}"))
}

/// Own process group so the terminal's Ctrl-C reaches `ser`, which forwards it, instead of the child directly.
pub fn spawn_foreground(command: &str, cwd: &Path, port: u16) -> Result<Child> {
    shell(command, cwd, port)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .with_context(|| format!("spawning: {command}"))
}

fn signal_group(pid: u32, signal: i32) {
    unsafe {
        libc::kill(-(pid as i32), signal);
        libc::kill(pid as i32, signal);
    }
}

/// Stops the session's process group. Does nothing when the pid no longer belongs to the session. Returns whether it was alive.
pub fn terminate(pid: u32, started: &str, grace: Duration) -> bool {
    if !is_alive(pid, started) {
        return false;
    }
    signal_group(pid, libc::SIGTERM);
    let deadline = Instant::now() + grace;
    while is_alive(pid, started) && Instant::now() < deadline {
        sleep(POLL);
    }
    if is_alive(pid, started) {
        signal_group(pid, libc::SIGKILL);
    }
    true
}

pub fn wait_for_port(child: &mut Child, port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if ports::accepts_connections(port) {
            return true;
        }
        if matches!(child.try_wait(), Ok(Some(_))) || Instant::now() >= deadline {
            return false;
        }
        sleep(POLL);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    Up,
    Unhealthy,
    Unreachable,
    Dead,
}

impl Liveness {
    pub fn label(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Unhealthy => "unhealthy",
            Self::Unreachable => "unreachable",
            Self::Dead => "dead",
        }
    }
}
