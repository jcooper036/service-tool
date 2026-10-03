use std::process::ExitCode;

use clap::Parser;

use service_tool::cli::Cli;
use service_tool::commands::{self, Silent};

fn main() -> ExitCode {
    match commands::run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if err.is::<Silent>() => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
