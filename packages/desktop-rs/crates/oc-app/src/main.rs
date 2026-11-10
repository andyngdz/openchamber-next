use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use oc_app::Launch;

/// OpenChamber desktop (Rust + GPUI spike).
#[derive(Parser)]
#[command(name = "oc-app")]
struct Cli {
    /// Project directory to open; defaults to the current directory.
    #[arg(long, default_value = ".")]
    directory: String,
    /// Write the layout of one rendered frame to this file and exit.
    #[arg(long, requires = "state")]
    dump_layout: Option<PathBuf>,
    /// State fixture file, e.g. fixtures/states/empty.json: the state --dump-layout renders,
    /// or, alone, the state the window opens on for review.
    #[arg(long)]
    state: Option<PathBuf>,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    let launch = match (cli.dump_layout, cli.state) {
        (Some(output), Some(state)) => Launch::DumpLayout { output, state },
        (None, Some(state)) => Launch::Fixture { state },
        (None | Some(_), None) => Launch::Window {
            directory: cli.directory,
        },
    };
    match oc_app::run(launch) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            oc_app::log_failure(&error);
            ExitCode::FAILURE
        }
    }
}
