use std::time::Duration;

use serde::Serialize;

use crate::db::Session;
use crate::ports;
use crate::process::{self, Liveness};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    pub url: String,
    pub ok: bool,
    pub detail: String,
}

pub fn probe(url: &str) -> HealthReport {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(PROBE_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    let (ok, detail) = match agent.get(url).call() {
        Ok(response) => {
            let status = response.status().as_u16();
            ((200..300).contains(&status), format!("HTTP {status}"))
        }
        Err(err) => (false, err.to_string()),
    };
    HealthReport {
        url: url.to_string(),
        ok,
        detail,
    }
}

pub fn health_url(session: &Session) -> Option<String> {
    session
        .health_url
        .as_ref()
        .map(|url| url.replace("{port}", &session.port.to_string()))
}

pub fn state(session: &Session) -> Liveness {
    if !process::is_alive(session.process_id, &session.process_started) {
        Liveness::Dead
    } else if !ports::accepts_connections(session.port) {
        Liveness::Unreachable
    } else if health_url(session).is_some_and(|url| !probe(&url).ok) {
        Liveness::Unhealthy
    } else {
        Liveness::Up
    }
}
