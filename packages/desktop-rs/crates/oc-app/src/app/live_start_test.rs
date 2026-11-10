use std::path::{Path, PathBuf};

use super::real_directory;

/// A scratch directory holding `project/` and `link -> project`, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("oc-app-{name}-{}", std::process::id()));
        std::fs::create_dir_all(root.join("project")).expect("create project");
        std::os::unix::fs::symlink(root.join("project"), root.join("link")).expect("symlink");
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn a_symlinked_directory_resolves_to_its_real_path() {
    let scratch = Scratch::new("real-directory-link");
    let real = std::fs::canonicalize(scratch.0.join("project")).expect("real path");

    let resolved = real_directory(&scratch.0.join("link/")).expect("resolves");

    assert_eq!(resolved, real.to_string_lossy());
    assert!(!resolved.ends_with('/'));
}

#[test]
fn relative_and_root_directories_resolve() {
    assert_eq!(real_directory(Path::new("/")).expect("root"), "/");
    let current = std::fs::canonicalize(".").expect("cwd");
    let relative = real_directory(Path::new("src")).expect("relative");
    assert_eq!(relative, current.join("src").to_string_lossy());
}

#[test]
fn a_missing_directory_is_an_error() {
    assert!(real_directory(Path::new("/nonexistent-oc-app-directory")).is_err());
}
