use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use oc_app::Launch;

/// OpenChamber desktop (Rust + GPUI).
#[derive(Parser)]
#[command(name = "oc-app")]
struct Cli {
    /// Write the layout of one frame rendered in a headless window to this file and exit.
    #[arg(long, requires = "state", conflicts_with = "screenshot")]
    dump_layout: Option<PathBuf>,
    /// Render one fixture state in a headless window, write the frame to this PNG and exit.
    #[arg(long, requires = "state")]
    screenshot: Option<PathBuf>,
    /// State fixture file, e.g. fixtures/states/empty.json: the state --dump-layout or
    /// --screenshot renders, or, alone, the state the window opens on for review.
    #[arg(long)]
    state: Option<PathBuf>,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    let launch = match (cli.dump_layout, cli.screenshot, cli.state) {
        // clap rejects either output flag without --state, and both of them together.
        (_, _, None) => Launch::Window,
        (Some(output), _, Some(state)) => Launch::DumpLayout { output, state },
        (None, Some(output), Some(state)) => Launch::Screenshot { output, state },
        (None, None, Some(state)) => Launch::Fixture { state },
    };
    match oc_app::run(launch) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            oc_app::log_failure(&error);
            ExitCode::FAILURE
        }
    }
}
