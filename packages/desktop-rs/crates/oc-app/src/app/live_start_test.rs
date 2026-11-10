use std::path::Path;

use super::absolute_directory;

#[test]
fn directories_are_absolute_without_a_trailing_slash() {
    assert_eq!(
        absolute_directory(Path::new("/tmp/project/")).expect("absolute"),
        "/tmp/project"
    );
    assert_eq!(absolute_directory(Path::new("/")).expect("root"), "/");
    let current = std::env::current_dir().expect("cwd");
    let relative = absolute_directory(Path::new("crates")).expect("relative");
    assert_eq!(relative, current.join("crates").to_string_lossy());
}
