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
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(unique_path(dir.path(), "a.pdf"), dir.path().join("a.pdf"));
    std::fs::write(dir.path().join("a.pdf"), "").unwrap();
    assert_eq!(unique_path(dir.path(), "a.pdf"), dir.path().join("a (1).pdf"));
    std::fs::write(dir.path().join("a (1).pdf"), "").unwrap();
    assert_eq!(unique_path(dir.path(), "a.pdf"), dir.path().join("a (2).pdf"));
    std::fs::write(dir.path().join("README"), "").unwrap();
    assert_eq!(unique_path(dir.path(), "README"), dir.path().join("README (1)"));
}
