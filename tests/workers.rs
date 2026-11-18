mod common;

use std::process::{Command, Output};
use std::time::{Duration, Instant};

use common::{Env, SERVER, free_port, stdout, wait_until};

/// Listens on nothing, ever.
const WORKER: &str = "sleep 300";

fn services(env: &Env) -> Vec<serde_json::Value> {
    env.status_json()["services"].as_array().unwrap().clone()
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_str(&stdout(out)).unwrap()
}

fn start_worker(env: &Env, name: &str, extra: &[&str]) -> Output {
    let mut args = vec!["run", "-d", "--no-port", "--json", "-n", name];
    args.extend_from_slice(extra);
    args.push(WORKER);
    env.ser(&args)
}

fn kill_hard(pid: u64) {
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
        libc::kill(pid as i32, libc::SIGKILL);
    }
}

fn state_of(env: &Env, index: usize) -> String {
    services(env)[index]["state"].as_str().unwrap().to_string()
}

#[test]
fn detached_worker_starts_without_a_port_and_follows_the_full_lifecycle() {
    let env = Env::new();

    // The default --wait is 10s. A worker that waited for a port would take that long and then warn.
    let began = Instant::now();
    let out = start_worker(&env, "w", &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        began.elapsed() < Duration::from_secs(5),
        "waited for a port"
    );

    let session = json(&out);
    assert!(session["port"].is_null(), "{session}");
    assert!(session["url"].is_null(), "{session}");
    assert_eq!(session["state"], "up");
    let id = session["id"].as_str().unwrap().to_string();

    let all = services(&env);
    assert_eq!(all.len(), 1);
    assert!(all[0]["port"].is_null() && all[0]["url"].is_null());
    assert_eq!(all[0]["state"], "up");
    assert!(env.ser(&["status", "-q"]).status.success());

    assert_eq!(json(&start_worker(&env, "w", &[]))["id"], id.as_str());
    assert_eq!(services(&env).len(), 1);

    let port = env.ser(&["port", "w"]);
    assert!(!port.status.success());
    let shown: serde_json::Value = serde_json::from_str(&env.ok(&["show", "w", "--json"])).unwrap();
    assert!(shown["session"]["port"].is_null() && shown["session"]["url"].is_null());

    let before = all[0]["process_id"].as_u64().unwrap();
    let restarted = env.ok(&["restart", "w"]);
    let restarted = restarted.trim();
    let after = services(&env);
    assert_eq!(after.len(), 1);
    assert_eq!(after[0]["id"], restarted);
    assert!(after[0]["port"].is_null() && after[0]["url"].is_null());
    assert_eq!(after[0]["state"], "up");
    let pid = after[0]["process_id"].as_u64().unwrap();
    assert_ne!(pid, before);

    kill_hard(pid);
    wait_until("worker to show dead", || state_of(&env, 0) == "dead");
    assert!(!env.ser(&["status", "-q"]).status.success());
    env.ok(&["heal"]);
    let after = services(&env);
    assert_eq!(after.len(), 1);
    assert!(after[0]["port"].is_null() && after[0]["url"].is_null());
    assert_eq!(after[0]["state"], "up");
    assert_eq!(after[0]["heal_count"], 1);
    assert_ne!(after[0]["process_id"].as_u64().unwrap(), pid);

    let live = after[0]["process_id"].as_u64().unwrap();
    env.ok(&["stop", "w"]);
    assert!(services(&env).is_empty());
    wait_until("worker process to exit", || unsafe {
        libc::kill(live as i32, 0) == -1
    });
    let stopped: serde_json::Value =
        serde_json::from_str(&env.ok(&["status", "--json", "--all"])).unwrap();
    let stopped = stopped["services"].as_array().unwrap();
    assert!(
        stopped
            .iter()
            .all(|s| s["state"] == "stopped" && s["port"].is_null())
    );
}

#[test]
fn worker_that_exits_immediately_is_an_error_not_a_started_session() {
    let env = Env::new();
    let out = env.ser(&[
        "run",
        "-d",
        "--no-port",
        "-n",
        "bad",
        "echo boom-from-worker; exit 3",
    ]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success()
            && err.contains("exited right after starting")
            && err.contains("boom-from-worker"),
        "{err}"
    );
    assert!(services(&env).is_empty());
    let all: serde_json::Value =
        serde_json::from_str(&env.ok(&["status", "--json", "--all"])).unwrap();
    assert_eq!(all["services"][0]["state"], "stopped");
}

#[test]
fn worker_gets_no_port_environment_variable() {
    let env = Env::new();
    let out = Command::new(env!("CARGO_BIN_EXE_ser"))
        .env("SER_HOME", env.root.path())
        .env("PORT", "4242")
        .current_dir(env.work.path())
        .args([
            "run",
            "-d",
            "--no-port",
            "-n",
            "w",
            "echo \"port=[${PORT-unset}]\"; sleep 300",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    wait_until("worker output", || {
        stdout(&env.ser(&["log", "w"])).contains("port=[")
    });
    assert!(stdout(&env.ser(&["log", "w"])).contains("port=[unset]"));
}

#[test]
fn worker_health_check_is_optional_and_needs_no_port() {
    let env = Env::new();
    let port = free_port();
    env.ok(&["run", "-d", "-n", "backend", SERVER, &port.to_string()]);
    let health = format!("http://127.0.0.1:{port}/");

    let out = start_worker(&env, "w", &["--health", &health]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(json(&out)["state"], "up");
    env.ok(&["check", "w"]);

    env.ok(&["stop", "backend"]);
    let states: Vec<_> = services(&env).iter().map(|s| s["state"].clone()).collect();
    assert_eq!(states, ["unhealthy"]);
    assert!(!env.ser(&["check", "w"]).status.success());
}

#[test]
fn worker_settings_that_need_a_port_are_rejected() {
    let env = Env::new();
    let refused = |args: &[&str]| {
        let out = env.ser(args);
        assert!(
            !out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    refused(&["run", "-d", "--no-port", "-p", "3000", WORKER]);
    refused(&["run", "-d", "--no-port", WORKER, "3000"]);
    refused(&["run", "-d", "--no-port", "--wait", "1", WORKER]);
    refused(&["run", "-d", "--no-port", "sleep 300 # {port}"]);
    refused(&[
        "run",
        "-d",
        "--no-port",
        "--health",
        "http://127.0.0.1:{port}/",
        WORKER,
    ]);
    assert!(
        services(&env).is_empty(),
        "a rejected worker must not start"
    );

    assert!(start_worker(&env, "w", &[]).status.success());
    refused(&["set", "w", "--health", "http://127.0.0.1:{port}/"]);
    assert!(services(&env)[0]["health_url"].is_null());
    env.ok(&["set", "w", "--health", "http://127.0.0.1:1/"]);
    assert_eq!(services(&env)[0]["health_url"], "http://127.0.0.1:1/");
}

#[test]
fn a_name_cannot_switch_between_worker_and_service_while_running() {
    let env = Env::new();
    let port = free_port().to_string();

    assert!(start_worker(&env, "w", &[]).status.success());
    let out = env.ser(&["run", "-d", "--wait", "0", "-n", "w", WORKER, &port]);
    assert!(
        !out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    env.ok(&["stop", "w"]);

    env.ok(&["run", "-d", "--wait", "0", "-n", "s", WORKER, &port]);
    let out = start_worker_named(&env, "s");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{err}");
}

fn start_worker_named(env: &Env, name: &str) -> Output {
    env.ser(&["run", "-d", "--no-port", "-n", name, WORKER])
}

/// The sessions table as it was before workers existed: ports required.
const LEGACY_SCHEMA: &str = "CREATE TABLE sessions (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        tags TEXT NOT NULL,
        cwd TEXT NOT NULL,
        command TEXT NOT NULL,
        health_url TEXT,
        requested_port INTEGER NOT NULL,
        port INTEGER NOT NULL,
        process_id INTEGER NOT NULL,
        process_started TEXT NOT NULL,
        created_at TEXT NOT NULL,
        stopped_at TEXT,
        log_path TEXT NOT NULL,
        git_branch TEXT,
        git_root TEXT,
        git_worktree INTEGER NOT NULL,
        heal_count INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX sessions_active_idx ON sessions (stopped_at);
    CREATE TABLE migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL);
    INSERT INTO migrations VALUES ('26100220_create_sessions', '2026-10-02T00:00:00+00:00');";

#[test]
fn sessions_from_before_workers_survive_the_migration_unchanged() {
    let env = Env::new();
    let mut live = Command::new("sleep").arg("300").spawn().unwrap();
    let port = free_port();
    let cwd = env.work.path().to_string_lossy().into_owned();

    let conn = rusqlite::Connection::open(env.root.path().join("ser.db")).unwrap();
    conn.execute_batch(LEGACY_SCHEMA).unwrap();
    let insert = "INSERT INTO sessions VALUES (?1, ?2, 'old,tag', ?3, 'cmd {port}', \
        'http://127.0.0.1:{port}/', ?4, ?4, ?5, '', '2026-10-02T00:00:00+00:00', ?6, '/tmp/x.log', \
        'main', '/repo', 1, 2)";
    conn.execute(
        insert,
        rusqlite::params![
            "0192aaaa-0000-7000-8000-000000000001",
            "legacy-live",
            cwd,
            port,
            live.id(),
            Option::<String>::None
        ],
    )
    .unwrap();
    conn.execute(
        insert,
        rusqlite::params![
            "0192aaaa-0000-7000-8000-000000000002",
            "legacy-stopped",
            cwd,
            port + 1,
            1,
            "2026-10-02T01:00:00+00:00"
        ],
    )
    .unwrap();
    drop(conn);

    let all: serde_json::Value =
        serde_json::from_str(&env.ok(&["status", "--json", "--all"])).unwrap();
    let rows = all["services"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let find = |name: &str| rows.iter().find(|r| r["name"] == name).unwrap();
    let (running, stopped) = (find("legacy-live"), find("legacy-stopped"));
    assert_eq!(running["port"], port);
    assert_eq!(running["url"], format!("http://localhost:{port}"));
    assert_eq!(running["state"], "unreachable");
    assert_eq!(running["tags"], serde_json::json!(["old", "tag"]));
    assert_eq!(running["heal_count"], 2);
    assert_eq!(running["git_branch"], "main");
    assert_eq!(running["health_url"], "http://127.0.0.1:{port}/");
    assert_eq!(stopped["port"], port + 1);
    assert_eq!(stopped["state"], "stopped");

    // The migrated database takes workers next to the old rows, and the old rows keep working.
    let worker = start_worker(&env, "w", &[]);
    assert!(
        worker.status.success(),
        "{}",
        String::from_utf8_lossy(&worker.stderr)
    );
    assert!(json(&worker)["port"].is_null());
    let ports: Vec<_> = services(&env).iter().map(|s| s["port"].clone()).collect();
    assert_eq!(ports.len(), 2);
    assert!(ports.contains(&serde_json::json!(port)) && ports.contains(&serde_json::Value::Null));

    env.ok(&["stop", "legacy-live"]);
    live.wait().unwrap();
    let conn = rusqlite::Connection::open(env.root.path().join("ser.db")).unwrap();
    let applied: i64 = conn
        .query_row("SELECT COUNT(*) FROM migrations", [], |row| row.get(0))
        .unwrap();
    assert_eq!(applied, 2);
}
