use super::{is_port_in_use_message, parse_port_line, pick_free_port};
use crate::errors::PortLineError;

#[test]
fn parses_a_plain_port_line() {
    assert_eq!(parse_port_line("57123\n"), Ok(57123));
    assert_eq!(parse_port_line("  4096  "), Ok(4096));
}

#[test]
fn rejects_a_password_line_without_keeping_the_password() {
    assert_eq!(
        parse_port_line("57123 pass:secret\n"),
        Err(PortLineError::PasswordLine)
    );
    assert_eq!(parse_port_line("pass:x"), Err(PortLineError::PasswordLine));
}

#[test]
fn rejects_garbage() {
    for line in [
        "hello",
        "0",
        "70000",
        "-1",
        "4096 4097",
        "Starting OpenChamber",
    ] {
        assert_eq!(
            parse_port_line(line),
            Err(PortLineError::NotAPort {
                line: line.to_owned()
            }),
            "line {line:?}"
        );
    }
}

#[test]
fn rejects_an_empty_line() {
    assert_eq!(parse_port_line(""), Err(PortLineError::Empty));
    assert_eq!(parse_port_line(" \n"), Err(PortLineError::Empty));
}

#[test]
fn detects_the_port_in_use_message() {
    let stderr = vec![
        "some warning".to_owned(),
        "Error: Port 4100 is already in use by another process.".to_owned(),
    ];
    assert!(is_port_in_use_message(&stderr));
}

#[test]
fn other_startup_errors_are_not_port_in_use() {
    let stderr = vec![
        "Error: OpenChamber is already running on port 4100 (PID: 12)".to_owned(),
        "Error: Port 4100 is used by OpenChamber Desktop app.".to_owned(),
    ];
    assert!(!is_port_in_use_message(&stderr));
    assert!(!is_port_in_use_message(&[]));
}

#[test]
fn picks_a_bindable_port() {
    let port = pick_free_port().expect("free port");
    std::net::TcpListener::bind(("127.0.0.1", port)).expect("port is free again");
}
