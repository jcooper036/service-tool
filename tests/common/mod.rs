#![allow(dead_code)]

use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU16, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

use tempfile::TempDir;

pub const SERVER: &str = "python3 -m http.server {port} --bind 127.0.0.1";

pub struct Env {
    pub root: TempDir,
    pub work: TempDir,
}

impl Env {
    pub fn new() -> Self {
        Self {
            root: TempDir::new().unwrap(),
            work: TempDir::new().unwrap(),
        }
    }

    pub fn ser(&self, args: &[&str]) -> Output {
        self.ser_in(self.work.path(), args)
    }

    pub fn ser_in(&self, dir: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ser"))
            .env("SER_HOME", self.root.path())
            .current_dir(dir)
            .args(args)
            .output()
            .unwrap()
    }

    pub fn ok(&self, args: &[&str]) -> String {
        let out = self.ser(args);
        assert!(
            out.status.success(),
            "ser {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        stdout(&out)
    }

    pub fn status_json(&self) -> serde_json::Value {
        serde_json::from_str(&self.ok(&["status", "--json"])).unwrap()
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = self.ser(&["stop", "--all"]);
    }
}

pub fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

static NEXT_PORT: AtomicU16 = AtomicU16::new(0);

/// Ports are spaced 10 apart so that a service bumped to the next port never lands on one another test is about to use.
pub fn free_port() -> u16 {
    loop {
        let mut next = NEXT_PORT.fetch_add(10, Ordering::SeqCst);
        if next == 0 {
            let base = 20000 + (std::process::id() % 2000) as u16 * 10;
            let _ = NEXT_PORT.compare_exchange(10, base + 10, Ordering::SeqCst, Ordering::SeqCst);
            next = base;
        }
        if next >= 20000 && TcpListener::bind(("127.0.0.1", next)).is_ok() {
            return next;
        }
    }
}

pub fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if cond() {
            return;
        }
        sleep(Duration::from_millis(100));
    }
    panic!("timed out waiting for {what}");
}
