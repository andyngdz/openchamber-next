use std::net::{Ipv4Addr, TcpListener};
use std::num::NonZeroU16;

use crate::errors::{PortLineError, SidecarError};

/// What the CLI prints on stderr when `--port` names a port another process
/// holds, or another OpenChamber instance answers on it
/// (`packages/web/bin/lib/commands-serve.js:96-99`), and what Node reports when
/// the bind itself loses the race (`listen EADDRINUSE`), which can land only
/// in the server log.
const PORT_TAKEN_MARKERS: [&str; 3] = [
    "is already in use by another process.",
    "OpenChamber is already running on port",
    "EADDRINUSE",
];

/// Token that starts the password half of a `<port> pass:<pw>` line.
const PASSWORD_PREFIX: &str = "pass:";

/// Picks a free port by binding `127.0.0.1:0` and closing the socket again.
///
/// Another process can take the port before the CLI binds it; the caller
/// retries on the CLI's "already in use" message.
///
/// # Examples
///
/// ```
/// let port = oc_sidecar::pick_free_port().expect("a free port");
/// assert!(port > 0);
/// ```
pub fn pick_free_port() -> Result<u16, SidecarError> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(SidecarError::FreePort)?;
    let address = listener.local_addr().map_err(SidecarError::FreePort)?;
    Ok(address.port())
}

/// Parses the first stdout line of `serve --foreground --quiet`.
///
/// The line is `<port>`; a `<port> pass:<pw>` line is rejected because the
/// sidecar runs without a UI password, and the password is not kept.
///
/// # Examples
///
/// ```
/// use oc_sidecar::{parse_port_line, PortLineError};
/// assert_eq!(parse_port_line("4096\n"), Ok(4096));
/// assert_eq!(parse_port_line("4096 pass:x"), Err(PortLineError::PasswordLine));
/// ```
pub fn parse_port_line(line: &str) -> Result<u16, PortLineError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(PortLineError::Empty);
    }
    let mut tokens = trimmed.split_whitespace();
    if tokens
        .clone()
        .any(|token| token.starts_with(PASSWORD_PREFIX))
    {
        return Err(PortLineError::PasswordLine);
    }
    let not_a_port = || PortLineError::NotAPort {
        line: trimmed.to_owned(),
    };
    let (Some(first), None) = (tokens.next(), tokens.next()) else {
        return Err(not_a_port());
    };
    first
        .parse::<NonZeroU16>()
        .map(NonZeroU16::get)
        .map_err(|_| not_a_port())
}

/// Whether CLI output lines (stderr or the log tail) say the picked port was
/// taken, which is the one startup failure worth a retry on a new port.
///
/// # Examples
///
/// ```
/// let stderr = ["Error: Port 4096 is already in use by another process.".to_owned()];
/// assert!(oc_sidecar::is_port_in_use_message(&stderr));
/// ```
pub fn is_port_in_use_message(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        PORT_TAKEN_MARKERS
            .iter()
            .any(|marker| line.contains(marker))
    })
}

#[cfg(test)]
#[path = "port_test.rs"]
mod port_test;
