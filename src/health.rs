use std::borrow::Cow;
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

/// The health URL with `{port}` filled in. A worker has no port to fill in.
pub fn health_url(session: &Session) -> Option<Cow<'_, str>> {
    let url = session.health_url.as_deref()?;
    Some(match session.port {
        Some(port) => Cow::Owned(url.replace("{port}", &port.to_string())),
        None => Cow::Borrowed(url),
    })
}

/// A worker is up when its process is alive (and its health check, if any, passes). It has no port to be unreachable on.
pub fn state(session: &Session) -> Liveness {
    if !process::is_alive(session.process_id, &session.process_started) {
        Liveness::Dead
    } else if session
        .port
        .is_some_and(|port| !ports::accepts_connections(port))
    {
        Liveness::Unreachable
    } else if health_url(session).is_some_and(|url| !probe(&url).ok) {
        Liveness::Unhealthy
    } else {
        Liveness::Up
    }
}
