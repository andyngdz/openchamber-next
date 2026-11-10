use std::net::{Ipv4Addr, TcpListener};
use std::num::NonZeroU16;

use crate::errors::{PortLineError, SidecarError};

/// What the CLI prints on stderr when `--port` names a port another process
/// holds (`packages/web/bin/lib/commands-serve.js:88-100`).
const PORT_IN_USE_MARKER: &str = "is already in use by another process.";

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

/// Whether the CLI's stderr says the picked port was taken, which is the one
/// startup failure worth a retry on a new port.
///
/// # Examples
///
/// ```
/// let stderr = ["Error: Port 4096 is already in use by another process.".to_owned()];
/// assert!(oc_sidecar::is_port_in_use_message(&stderr));
/// ```
pub fn is_port_in_use_message(stderr_lines: &[String]) -> bool {
    stderr_lines
        .iter()
        .any(|line| line.contains(PORT_IN_USE_MARKER))
}

#[cfg(test)]
#[path = "port_test.rs"]
mod port_test;
