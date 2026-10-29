use super::*;

#[test]
fn name_comes_from_engine_suggestion_then_url() {
    let url = Url::parse("https://a.com/remote.php/dav/files/me/My%20File.pdf").unwrap();
    assert_eq!(suggested_name(Path::new("/tmp/x.zip"), &url), "x.zip");
    assert_eq!(suggested_name(Path::new(""), &url), "My File.pdf");
    assert_eq!(suggested_name(Path::new(""), &Url::parse("https://a.com/").unwrap()), "download");
}

#[test]
fn sanitize_strips_path_tricks() {
    assert_eq!(sanitize("../evil:name?.txt"), "_evil_name_.txt");
    assert_eq!(sanitize(".hidden"), "hidden");
    assert_eq!(sanitize("   "), "download");
}

#[test]
fn unique_path_numbers_duplicates() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path();
    assert_eq!(unique_path(folder, "a.pdf"), folder.join("a.pdf"));
    std::fs::write(folder.join("a.pdf"), "").unwrap();
    assert_eq!(unique_path(folder, "a.pdf"), folder.join("a (1).pdf"));
    std::fs::write(folder.join("a (1).pdf"), "").unwrap();
    assert_eq!(unique_path(folder, "a.pdf"), folder.join("a (2).pdf"));
    std::fs::write(folder.join("README"), "").unwrap();
    assert_eq!(unique_path(folder, "README"), folder.join("README (1)"));
}

#[test]
fn destination_creates_a_missing_folder() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("Downloads").join("nested");
    let path = prepare_destination(&folder, "../report?.pdf");
    assert_eq!(path, folder.join("_report_.pdf"));
    assert!(folder.is_dir());
}

#[test]
fn short_transfer_is_a_failure() {
    assert_eq!(outcome(None, 1_000_000, 1_000), Err("incomplete: received 1000 of 1000000 bytes".into()));
    assert_eq!(outcome(None, 1_000, 1_000), Ok(()));
    assert_eq!(outcome(None, 0, 1_000), Ok(()));
    assert_eq!(outcome(Some("network".into()), 1_000, 1_000), Err("network".into()));
}

#[test]
fn discarding_removes_only_existing_partial_files() {
    let directory = tempfile::tempdir().unwrap();
    let partial = directory.path().join("a.bin");
    std::fs::write(&partial, "x").unwrap();
    discard_partial(&partial);
    assert!(!partial.exists());
    discard_partial(&partial);
}
