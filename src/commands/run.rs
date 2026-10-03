use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};

use crate::cli::RunArgs;
use crate::commands::{Silent, current_dir, view};
use crate::ctx::Ctx;
use crate::db::{self, Session};
use crate::output::print_json;
use crate::service::{self, Mode, Outcome, StartSpec};
use crate::{ports, process};

const DEFAULT_PORT: u16 = 3000;
const STOP_GRACE: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(100);

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
        unsafe {
            libc::signal(
                signal,
                on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t,
            );
        }
    }
}

fn report(args: &RunArgs, session: &Session) -> Result<()> {
    if args.json {
        return print_json(&view(session));
    }
    println!("{}", session.port);
    Ok(())
}

fn announce(verb: &str, session: &Session) {
    eprintln!(
        "{verb} {} on http://localhost:{} (session {})",
        session.name,
        session.port,
        session.short_id()
    );
}

fn tee<R: Read + Send + 'static>(mut source: R, mut sink: Box<dyn Write + Send>, log_path: String) {
    thread::spawn(move || {
        let mut log = OpenOptions::new().append(true).open(&log_path).ok();
        let mut buf = [0u8; 8192];
        loop {
            match source.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let _ = sink.write_all(&buf[..n]);
                    let _ = sink.flush();
                    if let Some(log) = log.as_mut() {
                        let _ = log.write_all(&buf[..n]);
                    }
                }
            }
        }
    });
}

fn foreground(ctx: &Ctx, session: &Session, mut child: Child, wait: Duration) -> Result<()> {
    announce("running", session);
    if let Some(out) = child.stdout.take() {
        tee(out, Box::new(std::io::stdout()), session.log_path.clone());
    }
    if let Some(err) = child.stderr.take() {
        tee(err, Box::new(std::io::stderr()), session.log_path.clone());
    }
    install_signal_handlers();

    let began = Instant::now();
    let (mut listening, mut warned) = (false, false);
    let mut stopped_by_us = false;
    let status = loop {
        if INTERRUPTED.load(Ordering::SeqCst) {
            process::terminate(session.process_id, &session.process_started, STOP_GRACE);
            stopped_by_us = true;
            break child.wait()?;
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if !listening && !warned {
            if ports::accepts_connections(session.port) {
                listening = true;
            } else if began.elapsed() > wait {
                warned = true;
                eprintln!(
                    "warning: nothing is listening on port {} yet. The command must use {{port}} or $PORT",
                    session.port
                );
            }
        }
        sleep(POLL);
    };
    sleep(Duration::from_millis(150));
    db::mark_stopped(&ctx.conn, &session.id)?;

    if stopped_by_us || status.signal().is_some() {
        eprintln!("stopped {}", session.name);
        return Ok(());
    }
    if status.success() {
        eprintln!("{} exited", session.name);
        return Ok(());
    }
    eprintln!("{} exited with {status}", session.name);
    bail!(Silent)
}

pub fn run(ctx: &mut Ctx, args: RunArgs) -> Result<()> {
    let command = match (&args.command, &args.command_flag) {
        (Some(c), None) | (None, Some(c)) => c.clone(),
        _ => bail!("give the command to run, as the first argument or with -c"),
    };
    let requested = args.port.or(args.port_flag).unwrap_or(DEFAULT_PORT);
    if requested == 0 {
        bail!("port must be 1-65535");
    }
    let cwd = current_dir()?;
    let name = match &args.name {
        Some(name) => name.clone(),
        None => cwd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "service".to_string()),
    };
    let tags = args
        .tags
        .iter()
        .flat_map(|t| t.split(','))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect();
    let mode = if args.detached {
        Mode::Detached
    } else {
        Mode::Foreground
    };
    let spec = StartSpec {
        name,
        tags,
        cwd,
        command,
        health_url: args.health.clone(),
        requested_port: requested,
        heal_count: 0,
        mode,
    };
    let wait = Duration::from_secs(args.wait);

    match service::start(ctx, &spec)? {
        Outcome::Existing(session) => {
            announce("already running:", &session);
            report(&args, &session)
        }
        Outcome::Started(session, child) if mode == Mode::Foreground => {
            foreground(ctx, &session, child, wait)
        }
        Outcome::Started(session, mut child) => {
            if !service::await_up(ctx, &session, &mut child, wait)? {
                eprintln!(
                    "warning: nothing is listening on port {} after {}s. The command must use {{port}} or $PORT. See `ser show {}`",
                    session.port, args.wait, session.name
                );
            }
            announce("started", &session);
            report(&args, &session)
        }
    }
}
