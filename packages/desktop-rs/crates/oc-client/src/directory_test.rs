use super::Directory;

#[test]
fn a_directory_keeps_the_exact_path_it_was_given() {
    let directory = Directory::new("/tmp/my project/");

    assert_eq!(directory.as_str(), "/tmp/my project/");
    assert_eq!(directory.to_string(), "/tmp/my project/");
}

#[test]
fn a_directory_converts_from_owned_and_borrowed_paths() {
    assert_eq!(Directory::from("/a").as_str(), "/a");
    assert_eq!(Directory::from("/b".to_owned()).as_str(), "/b");
}
