mod common;

use std::io::Read;
use std::process::{Command, Stdio};

use common::{Env, SERVER, free_port, wait_until};

fn kill_hard(pid: u64) {
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
        libc::kill(pid as i32, libc::SIGKILL);
    }
}

fn first_pid(env: &Env) -> u64 {
    env.status_json()["services"][0]["process_id"]
        .as_u64()
        .unwrap()
}

#[test]
fn heal_revives_a_crashed_service_but_not_a_stopped_one() {
    let env = Env::new();
    let port = free_port().to_string();
    env.ok(&["run", "-d", "-n", "crashy", SERVER, &port]);
    env.ok(&[
        "run",
        "-d",
        "-n",
        "stopped",
        SERVER,
        &free_port().to_string(),
    ]);
    env.ok(&["stop", "stopped"]);

    let pid = first_pid(&env);
    kill_hard(pid);
    wait_until("service to show dead", || {
        env.status_json()["services"][0]["state"] == "dead"
    });

    let healed = env.ok(&["heal"]);
    assert!(healed.contains("healed crashy"), "{healed}");
    let json = env.status_json();
    let services = json["services"].as_array().unwrap();
    assert_eq!(services.len(), 1, "stopped service must stay stopped");
    assert_eq!(services[0]["name"], "crashy");
    assert_eq!(services[0]["state"], "up");
    assert_eq!(services[0]["port"].to_string(), port);
    assert_ne!(services[0]["process_id"].as_u64().unwrap(), pid);
    assert_eq!(services[0]["heal_count"], 1);
}

#[test]
fn heal_gives_up_after_repeated_crashes() {
    let env = Env::new();
    env.ok(&[
        "run",
        "-d",
        "-n",
        "crashy",
        SERVER,
        &free_port().to_string(),
    ]);
    for round in 1..=3 {
        kill_hard(first_pid(&env));
        wait_until("dead", || {
            env.status_json()["services"][0]["state"] == "dead"
        });
        assert!(env.ok(&["heal"]).contains("healed"), "round {round}");
    }
    kill_hard(first_pid(&env));
    wait_until("dead", || {
        env.status_json()["services"][0]["state"] == "dead"
    });
    let out = env.ok(&["heal"]);
    assert!(
        out.contains("skipped crashy") && out.contains("ser restart"),
        "{out}"
    );

    env.ok(&["restart", "crashy"]);
    assert_eq!(env.status_json()["services"][0]["heal_count"], 0);
}

#[test]
fn heal_skips_a_service_whose_directory_is_gone() {
    let env = Env::new();
    let dir = env.work.path().join("gone");
    std::fs::create_dir(&dir).unwrap();
    env.ser_in(&dir, &["run", "-d", SERVER, &free_port().to_string()]);
    kill_hard(first_pid(&env));
    std::fs::remove_dir_all(&dir).unwrap();
    wait_until("dead", || {
        env.status_json()["services"][0]["state"] == "dead"
    });
    assert!(env.ok(&["heal"]).contains("no longer exists"));
}

#[test]
fn stop_does_not_signal_a_reused_pid() {
    let env = Env::new();
    let mut bystander = Command::new("sleep").arg("60").spawn().unwrap();
    // Start times have one-second resolution; make sure the session starts in a later second.
    std::thread::sleep(std::time::Duration::from_millis(1200));
    let rendered = "sleep 60 # {port}";
    env.ok(&[
        "run",
        "-d",
        "--wait",
        "0",
        "-n",
        "fake",
        rendered,
        &free_port().to_string(),
    ]);

    // Point the session at the bystander's pid, as if the original pid had been recycled.
    let db = env.root.path().join("ser.db");
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.execute("UPDATE sessions SET process_id = ?1", [bystander.id()])
        .unwrap();
    drop(conn);

    env.ok(&["stop", "fake"]);
    assert!(
        bystander.try_wait().unwrap().is_none(),
        "bystander was killed"
    );
    bystander.kill().unwrap();
}

#[test]
fn foreground_run_tracks_streams_and_stops_on_ctrl_c() {
    let env = Env::new();
    let port = free_port();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ser"))
        .env("SER_HOME", env.root.path())
        .current_dir(env.work.path())
        .args([
            "run",
            "-n",
            "fg",
            "echo hello-foreground; python3 -u -m http.server {port} --bind 127.0.0.1",
            &port.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let out = env.ser(&["status", "--json"]);
        let last = String::from_utf8_lossy(&out.stdout).into_owned();
        if serde_json::from_str::<serde_json::Value>(&last)
            .map(|j| j["services"][0]["state"] == "up")
            .unwrap_or(false)
        {
            break;
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            let mut err = String::new();
            child
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut err)
                .unwrap();
            panic!("never came up\nstatus: {last}\nser stderr: {err}");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    unsafe { libc::kill(child.id() as i32, libc::SIGINT) };
    let status = child.wait().unwrap();
    assert!(status.success());

    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    assert!(out.contains("hello-foreground"), "{out}");
    let all: serde_json::Value =
        serde_json::from_str(&env.ok(&["status", "--json", "--all"])).unwrap();
    assert_eq!(all["services"][0]["state"], "stopped");
    assert!(env.ok(&["log", "fg"]).contains("hello-foreground"));
}

#[test]
fn unreachable_before_listening_and_heal_is_quiet_when_all_alive() {
    let env = Env::new();
    env.ok(&["run", "-d", "-n", "web", SERVER, &free_port().to_string()]);
    assert!(env.ok(&["heal"]).contains("nothing to heal"));
}
