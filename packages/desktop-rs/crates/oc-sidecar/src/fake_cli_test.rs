//! Fake CLI entry files for launch and sidecar tests: each is a one-line
//! Node script run in place of `packages/web/bin/cli.js`.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::config::SidecarConfig;

const PORT_ARG: &str = "const port = process.argv[process.argv.indexOf('--port') + 1];";
const STAY_ALIVE: &str = "setInterval(() => {}, 1000);";

/// Prints the requested port and runs until signalled.
pub(crate) fn prints_port() -> String {
    format!("{PORT_ARG} process.stdout.write(port + '\\n'); {STAY_ALIVE}")
}

/// Prints the requested port and ignores SIGTERM.
pub(crate) fn ignores_sigterm() -> String {
    format!("process.on('SIGTERM', () => {{}}); {}", prints_port())
}

/// Prints a password port line.
pub(crate) fn prints_password() -> String {
    format!("process.stdout.write('4000 pass:pw\\n'); {STAY_ALIVE}")
}

/// Fails the way the CLI does when the port is taken.
pub(crate) fn port_in_use() -> String {
    format!(
        "{PORT_ARG} console.error('Error: Port ' + port + ' is already in use by another process.'); process.exit(1);"
    )
}

/// Writes a line to the port's log file, then exits 3 with an error on stderr.
pub(crate) fn crashes_with_log() -> String {
    format!(
        "{PORT_ARG} const fs = require('fs'); const dir = process.env.OPENCHAMBER_DATA_DIR + '/logs'; fs.mkdirSync(dir, {{ recursive: true }}); fs.appendFileSync(dir + '/openchamber-' + port + '.log', 'boom in log\\n'); console.error('Error: boom'); process.exit(3);"
    )
}

/// Prints nothing and runs until signalled.
pub(crate) fn stays_silent() -> String {
    STAY_ALIVE.to_owned()
}

/// Writes `script` as a CLI file in a fresh temp dir and returns a config
/// whose data dir is that temp dir, with short timeouts.
pub(crate) fn fake_config(name: &str, script: &str) -> SidecarConfig {
    let dir = std::env::temp_dir().join(format!("oc-sidecar-fake-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("fake dir");
    let cli_path = dir.join("cli.js");
    fs::write(&cli_path, script).expect("fake cli");
    let mut config = SidecarConfig::new(
        cli_path,
        dir.clone(),
        vec![("OPENCHAMBER_DATA_DIR".into(), dir.into_os_string())],
    );
    config.first_line_timeout = Duration::from_secs(10);
    config.graceful_timeout = Duration::from_secs(5);
    config
}

/// Removes the temp dir of a config made by [`fake_config`].
pub(crate) fn remove_fake(config: &SidecarConfig) {
    fs::remove_dir_all(PathBuf::from(&config.working_dir)).ok();
}
