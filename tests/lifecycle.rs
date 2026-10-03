mod common;

use std::net::TcpListener;

use common::{Env, SERVER, free_port, stdout};

fn services(env: &Env) -> Vec<serde_json::Value> {
    env.status_json()["services"].as_array().unwrap().clone()
}

#[test]
fn run_detached_prints_port_and_tracks_service() {
    let env = Env::new();
    let port = free_port();
    let printed = env.ok(&["run", "-d", "-n", "web", SERVER, &port.to_string()]);
    assert_eq!(printed.trim(), port.to_string());

    let all = services(&env);
    assert_eq!(all.len(), 1);
    assert_eq!(all[0]["name"], "web");
    assert_eq!(all[0]["state"], "up");
    assert_eq!(all[0]["url"], format!("http://localhost:{port}"));
    let table = env.ok(&["status"]);
    assert!(table.contains(&format!("http://localhost:{port}")));

    env.ok(&["stop", "web"]);
    assert!(services(&env).is_empty());
    assert_eq!(services_all(&env).len(), 1);
}

fn services_all(env: &Env) -> Vec<serde_json::Value> {
    let json: serde_json::Value =
        serde_json::from_str(&env.ok(&["status", "--json", "--all"])).unwrap();
    json["services"].as_array().unwrap().clone()
}

#[test]
fn running_the_same_service_again_reuses_the_session() {
    let env = Env::new();
    let port = free_port().to_string();
    let first = env.ok(&["run", "-d", "-n", "web", SERVER, &port]);
    let second = env.ok(&["run", "-d", "-n", "web", SERVER, &port]);
    assert_eq!(first, second);
    assert_eq!(services(&env).len(), 1);
}

#[test]
fn same_name_with_a_different_command_is_an_error() {
    let env = Env::new();
    let port = free_port().to_string();
    env.ok(&["run", "-d", "-n", "web", SERVER, &port]);
    let out = env.ser(&["run", "-d", "-n", "web", "sleep 100 # {port}", &port]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("different command"));
}

#[test]
fn taken_port_moves_to_the_next_free_one() {
    let env = Env::new();
    let blocker = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let taken = blocker.local_addr().unwrap().port();
    let printed = env.ok(&["run", "-d", "-n", "web", SERVER, &taken.to_string()]);
    let got: u16 = printed.trim().parse().unwrap();
    assert!(got > taken, "got {got}, taken {taken}");
}

#[test]
fn services_do_not_share_ports() {
    let env = Env::new();
    let port = free_port().to_string();
    let a: u16 = env
        .ok(&["run", "-d", "-n", "a", SERVER, &port])
        .trim()
        .parse()
        .unwrap();
    let b: u16 = env
        .ok(&["run", "-d", "-n", "b", SERVER, &port])
        .trim()
        .parse()
        .unwrap();
    assert_ne!(a, b);
}

#[test]
fn port_env_var_is_enough() {
    let env = Env::new();
    let port = free_port().to_string();
    let printed = env.ok(&[
        "run",
        "-d",
        "-n",
        "envvar",
        "python3 -m http.server $PORT --bind 127.0.0.1",
        &port,
    ]);
    assert_eq!(printed.trim(), port);
    assert_eq!(services(&env)[0]["state"], "up");
}

#[test]
fn command_that_dies_early_is_reported_with_its_log() {
    let env = Env::new();
    let out = env.ser(&["run", "-d", "-n", "bad", "echo boom-from-service; exit 3"]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("exited before listening") && err.contains("boom-from-service"),
        "{err}"
    );
    assert!(services(&env).is_empty());
}

#[test]
fn health_check_and_show() {
    let env = Env::new();
    let port = free_port().to_string();
    env.ok(&[
        "run",
        "-d",
        "-n",
        "web",
        "--health",
        "http://127.0.0.1:{port}/",
        "-t",
        "api,demo",
        SERVER,
        &port,
    ]);
    assert!(env.ok(&["check", "web"]).starts_with("healthy"));
    let shown = env.ok(&["show", "web"]);
    assert!(
        shown.contains("state: up")
            && shown.contains("tags: api, demo")
            && shown.contains("health: ok")
    );
    let json: serde_json::Value =
        serde_json::from_str(&env.ok(&["show", "web", "--json"])).unwrap();
    assert_eq!(json["session"]["name"], "web");
    assert!(json["log_tail"].is_array());
}

#[test]
fn check_without_health_url_fails() {
    let env = Env::new();
    env.ok(&["run", "-d", "-n", "web", SERVER, &free_port().to_string()]);
    assert!(!env.ser(&["check", "web"]).status.success());
}

#[test]
fn port_command_prints_only_the_port() {
    let env = Env::new();
    let port = free_port().to_string();
    env.ok(&["run", "-d", "-n", "web", SERVER, &port]);
    assert_eq!(env.ok(&["port", "web"]).trim(), port);
    env.ok(&["stop", "web"]);
    assert!(!env.ser(&["port", "web"]).status.success());
}

#[test]
fn stop_by_tag_id_and_process_id() {
    let env = Env::new();
    env.ok(&[
        "run",
        "-d",
        "-n",
        "a",
        "-t",
        "group",
        SERVER,
        &free_port().to_string(),
    ]);
    env.ok(&[
        "run",
        "-d",
        "-n",
        "b",
        "-t",
        "group",
        SERVER,
        &free_port().to_string(),
    ]);
    env.ok(&["stop", "group"]);
    assert!(services(&env).is_empty());

    env.ok(&["run", "-d", "-n", "c", SERVER, &free_port().to_string()]);
    let id = services(&env)[0]["id"].as_str().unwrap().to_string();
    env.ok(&["stop", &id[id.len() - 8..]]);
    assert!(services(&env).is_empty());

    env.ok(&["run", "-d", "-n", "d", SERVER, &free_port().to_string()]);
    let pid = services(&env)[0]["process_id"]
        .as_u64()
        .unwrap()
        .to_string();
    env.ok(&["stop", &pid]);
    assert!(services(&env).is_empty());
}

#[test]
fn stop_with_unknown_target_fails() {
    let env = Env::new();
    assert!(!env.ser(&["stop", "ghost"]).status.success());
}

#[test]
fn restart_prefers_the_same_port_and_new_process() {
    let env = Env::new();
    let port = free_port().to_string();
    env.ok(&["run", "-d", "-n", "web", SERVER, &port]);
    let before = services(&env)[0]["process_id"].as_u64().unwrap();
    let printed = env.ok(&["restart", "web"]);
    assert_eq!(printed.trim(), port);
    let after = services(&env);
    assert_eq!(after.len(), 1);
    assert_ne!(after[0]["process_id"].as_u64().unwrap(), before);
}

#[test]
fn log_prints_service_output() {
    let env = Env::new();
    env.ok(&[
        "run",
        "-d",
        "-n",
        "web",
        "echo hello-log; python3 -m http.server {port} --bind 127.0.0.1",
        &free_port().to_string(),
    ]);
    assert!(stdout(&env.ser(&["log", "web"])).contains("hello-log"));
}

#[test]
fn prune_removes_stopped_sessions_only() {
    let env = Env::new();
    env.ok(&["run", "-d", "-n", "old", SERVER, &free_port().to_string()]);
    env.ok(&["stop", "old"]);
    env.ok(&["run", "-d", "-n", "live", SERVER, &free_port().to_string()]);
    assert!(env.ok(&["prune"]).contains("pruned 1"));
    assert_eq!(services_all(&env).len(), 1);
    assert_eq!(services(&env)[0]["name"], "live");
}

#[test]
fn quiet_status_reflects_health() {
    let env = Env::new();
    assert!(!env.ser(&["status", "-q"]).status.success());
    env.ok(&["run", "-d", "-n", "web", SERVER, &free_port().to_string()]);
    assert!(env.ser(&["status", "-q"]).status.success());
}

#[test]
fn unmanaged_listeners_are_reported() {
    let env = Env::new();
    let blocker = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = blocker.local_addr().unwrap().port();
    let json = env.status_json();
    let unmanaged = json["unmanaged"].as_array().unwrap();
    assert!(unmanaged.iter().any(|l| l["port"] == port), "{unmanaged:?}");
}

#[test]
fn run_requires_a_command() {
    let env = Env::new();
    assert!(!env.ser(&["run", "-d"]).status.success());
}

fn spawn_ser(env: &Env, args: &[&str]) -> std::process::Child {
    std::process::Command::new(env!("CARGO_BIN_EXE_ser"))
        .env("SER_HOME", env.root.path())
        .current_dir(env.work.path())
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
}

#[test]
fn concurrent_first_use_of_a_fresh_database_does_not_collide() {
    let env = Env::new();
    let children: Vec<_> = (0..6)
        .map(|_| spawn_ser(&env, &["status", "--json"]))
        .collect();
    for child in children {
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn concurrent_runs_never_claim_the_same_port() {
    let env = Env::new();
    let port = free_port().to_string();
    let children: Vec<_> = (0..4)
        .map(|i| {
            let name = format!("svc{i}");
            spawn_ser(&env, &["run", "-d", "-n", &name, SERVER, &port])
        })
        .collect();
    let mut ports = Vec::new();
    for child in children {
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        ports.push(stdout(&out).trim().to_string());
    }
    ports.sort();
    ports.dedup();
    assert_eq!(ports.len(), 4, "ports collided: {ports:?}");
    assert!(
        env.status_json()["services"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["state"] == "up")
    );
}
