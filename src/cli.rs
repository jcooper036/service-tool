use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ser",
    version,
    about = "Track the servers you run on localhost",
    arg_required_else_help = true
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        env = "SER_HOME",
        help = "State directory (default ~/.ser)"
    )]
    pub home: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Run a service in the foreground (or detached with -d) and track it")]
    Run(RunArgs),
    #[command(about = "Rebuild and reinstall ser from its source checkout")]
    Update,
    #[command(about = "Stop services by name, tag, session id, or process id")]
    Stop(StopArgs),
    #[command(about = "Show services, and listening ports ser does not manage")]
    Status(StatusArgs),
    #[command(about = "Restart services that died")]
    Heal(HealArgs),
    #[command(about = "Stop and relaunch a service, preferring its port")]
    Restart(RestartArgs),
    #[command(about = "Print a service's latest session log")]
    Log(LogArgs),
    #[command(about = "Run a service's health check")]
    Check(TargetArgs),
    #[command(about = "Diagnose a service: session, directory, branch, health, log tail")]
    Show(ShowArgs),
    #[command(about = "Print the port of a running service")]
    Port(TargetArgs),
    #[command(about = "Delete stopped sessions and their logs")]
    Prune,
}

#[derive(Args)]
pub struct RunArgs {
    #[arg(
        value_name = "COMMAND",
        conflicts_with = "command_flag",
        help = "Command to start the server. Use {port}, or read the PORT environment variable"
    )]
    pub command: Option<String>,
    #[arg(
        value_name = "PORT",
        conflicts_with = "port_flag",
        help = "Port to prefer"
    )]
    pub port: Option<u16>,
    #[arg(
        short = 'c',
        long = "command",
        id = "command_flag",
        value_name = "COMMAND"
    )]
    pub command_flag: Option<String>,
    #[arg(short = 'p', long = "port", id = "port_flag", value_name = "PORT")]
    pub port_flag: Option<u16>,
    #[arg(short, long, help = "Run on another process and return")]
    pub detached: bool,
    #[arg(short, long = "tag", help = "Tag (repeatable)")]
    pub tags: Vec<String>,
    #[arg(short, long, help = "Name (default: the directory name)")]
    pub name: Option<String>,
    #[arg(long, help = "Health URL containing {port}; a 2xx response is healthy")]
    pub health: Option<String>,
    #[arg(
        long,
        default_value_t = 10,
        help = "Seconds to wait for the port to open"
    )]
    pub wait: u64,
    #[arg(long, help = "Print the session as JSON instead of just the port")]
    pub json: bool,
}

#[derive(Args)]
pub struct StopArgs {
    #[arg(
        conflicts_with = "all",
        required_unless_present = "all",
        help = "Name, tag, session id, or process id"
    )]
    pub target: Option<String>,
    #[arg(short, long)]
    pub all: bool,
}

#[derive(Args)]
pub struct StatusArgs {
    #[arg(long, help = "Print JSON to stdout")]
    pub json: bool,
    #[arg(short, long, help = "Include stopped sessions")]
    pub all: bool,
    #[arg(
        short,
        long,
        conflicts_with = "json",
        help = "No output; exit 0 only if something is running and all of it is up"
    )]
    pub quiet: bool,
}

#[derive(Args)]
pub struct HealArgs {
    #[arg(long, help = "Print JSON to stdout")]
    pub json: bool,
    #[arg(long, default_value_t = 10, help = "Seconds to wait for each port")]
    pub wait: u64,
}

#[derive(Args)]
pub struct RestartArgs {
    #[arg(help = "Name or session id")]
    pub target: String,
    #[arg(long, default_value_t = 10, help = "Seconds to wait for the port")]
    pub wait: u64,
}

#[derive(Args)]
pub struct LogArgs {
    #[arg(help = "Name or session id")]
    pub target: String,
    #[arg(short = 'n', long = "lines", default_value_t = 100)]
    pub lines: usize,
    #[arg(short, long, help = "Keep printing appended lines")]
    pub follow: bool,
}

#[derive(Args)]
pub struct TargetArgs {
    #[arg(help = "Name or session id")]
    pub target: String,
}

#[derive(Args)]
pub struct ShowArgs {
    #[arg(help = "Name or session id")]
    pub target: String,
    #[arg(short = 'n', long = "lines", default_value_t = 20)]
    pub lines: usize,
    #[arg(long, help = "Print one JSON object")]
    pub json: bool,
}
