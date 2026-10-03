use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::process::Command;
use std::time::Duration;

use anyhow::{Result, bail};
use serde::Serialize;

const BIND_ADDRESSES: [IpAddr; 3] = [
    IpAddr::V4(Ipv4Addr::LOCALHOST),
    IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    IpAddr::V6(Ipv6Addr::LOCALHOST),
];

/// Free means bindable on every loopback and wildcard address and nothing answering on loopback.
/// An address the machine does not have (no IPv6) is not a conflict.
pub fn is_free(port: u16) -> bool {
    for addr in BIND_ADDRESSES {
        match TcpListener::bind((addr, port)) {
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::AddrNotAvailable => {}
            Err(_) => return false,
        }
    }
    !accepts_connections(port)
}

/// Dev servers often listen on only one of the two loopback families, so both are tried.
pub fn accepts_connections(port: u16) -> bool {
    [
        SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
        SocketAddr::from((Ipv6Addr::LOCALHOST, port)),
    ]
    .iter()
    .any(|addr| TcpStream::connect_timeout(addr, Duration::from_millis(300)).is_ok())
}

/// First free port at or above `requested` that no live session has claimed.
pub fn pick(requested: u16, claimed: &HashSet<u16>) -> Result<u16> {
    match (requested..=u16::MAX).find(|p| !claimed.contains(p) && is_free(*p)) {
        Some(port) => Ok(port),
        None => bail!("no free port at or above {requested}"),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Listener {
    pub port: u16,
    pub pid: u32,
    pub command: String,
}

/// TCP listeners on this machine according to lsof. Empty when lsof is unavailable.
pub fn listeners() -> Vec<Listener> {
    let Ok(output) = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"])
        .output()
    else {
        return Vec::new();
    };
    let mut found: Vec<Listener> = Vec::new();
    let (mut pid, mut command) = (0u32, String::new());
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let (tag, value) = line.split_at(1.min(line.len()));
        match tag {
            "p" => pid = value.parse().unwrap_or(0),
            "c" => command = value.to_string(),
            "n" => {
                let port = value
                    .rsplit_once(':')
                    .and_then(|(_, p)| p.parse::<u16>().ok());
                if let Some(port) = port
                    && !found.iter().any(|l| l.pid == pid && l.port == port)
                {
                    found.push(Listener {
                        port,
                        pid,
                        command: command.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    found.sort_by_key(|l| l.port);
    found
}
