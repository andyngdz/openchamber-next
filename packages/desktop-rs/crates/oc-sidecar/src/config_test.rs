use std::path::PathBuf;

use super::SidecarConfig;

fn config_with(extra_env: &[(&str, &str)]) -> SidecarConfig {
    SidecarConfig::new(
        PathBuf::from("cli.js"),
        PathBuf::from("/work"),
        extra_env
            .iter()
            .map(|(key, value)| ((*key).into(), (*value).into()))
            .collect(),
    )
}

#[test]
fn data_dir_comes_from_the_extra_env() {
    let config = config_with(&[
        ("OPENCHAMBER_DATA_DIR", "/tmp/a"),
        ("OPENCHAMBER_DATA_DIR", "/tmp/b"),
    ]);
    assert_eq!(config.data_dir(), Some(PathBuf::from("/tmp/b")));
}

#[test]
fn isolated_home_sets_the_default_data_dir() {
    let config = config_with(&[("OPENCHAMBER_DATA_DIR", ""), ("HOME", "/tmp/home")]);
    assert_eq!(
        config.data_dir(),
        Some(PathBuf::from("/tmp/home/.config/openchamber"))
    );
}
