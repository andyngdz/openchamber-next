use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use super::{LogCursor, last_lines, log_file_path, resolve_data_dir};

fn lines_text(lines: &[&str]) -> String {
    lines.iter().map(|line| format!("{line}\n")).collect()
}

#[test]
fn log_path_for_a_port_and_data_dir() {
    assert_eq!(
        log_file_path(Path::new("/tmp/oc-data"), 51234),
        PathBuf::from("/tmp/oc-data/logs/openchamber-51234.log")
    );
}

#[test]
fn data_dir_comes_from_the_env_value_first() {
    let dir = resolve_data_dir(
        Some(OsStr::new(" /srv/oc ")),
        Some(OsStr::new("/home/u")),
        Path::new("/work"),
    );
    assert_eq!(dir, Some(PathBuf::from("/srv/oc")));
}

#[test]
fn relative_data_dir_resolves_against_the_working_dir() {
    let dir = resolve_data_dir(Some(OsStr::new("data")), None, Path::new("/work"));
    assert_eq!(dir, Some(PathBuf::from("/work/data")));
}

#[test]
fn blank_data_dir_falls_back_to_home() {
    let dir = resolve_data_dir(
        Some(OsStr::new("  ")),
        Some(OsStr::new("/home/u")),
        Path::new("/work"),
    );
    assert_eq!(dir, Some(PathBuf::from("/home/u/.config/openchamber")));
}

#[test]
fn no_data_dir_and_no_home_is_unknown() {
    assert_eq!(resolve_data_dir(None, None, Path::new("/work")), None);
}

#[test]
fn keeps_the_last_lines() {
    let text = lines_text(&["a", "", "b", "c", "d"]);
    assert_eq!(last_lines(&text, 2), vec!["c".to_owned(), "d".to_owned()]);
    assert_eq!(last_lines(&text, 10).len(), 4);
}

#[test]
fn cursor_reports_only_lines_written_after_it() {
    let path = std::env::temp_dir().join(format!("oc-sidecar-log-{}.log", std::process::id()));
    fs::write(&path, lines_text(&["old line"])).expect("old content");
    let cursor = LogCursor::at_end_of(path.clone());
    fs::write(&path, lines_text(&["old line", "new line"])).expect("new content");
    assert_eq!(cursor.tail(), vec!["new line".to_owned()]);

    fs::write(&path, lines_text(&["rotated"])).expect("rotated content");
    assert_eq!(cursor.tail(), vec!["rotated".to_owned()]);
    fs::remove_file(&path).ok();
}
