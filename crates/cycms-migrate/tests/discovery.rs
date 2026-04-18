use std::fs;

use cycms_migrate::scan;
use tempfile::tempdir;

fn write_file(dir: &std::path::Path, name: &str, contents: &str) {
    fs::write(dir.join(name), contents).unwrap();
}

#[test]
fn scan_returns_empty_for_empty_directory() {
    let dir = tempdir().unwrap();
    let result = scan(dir.path()).unwrap();
    assert!(result.is_empty());
}

#[test]
fn scan_parses_up_and_down_pairs_sorted_by_version() {
    let dir = tempdir().unwrap();
    write_file(dir.path(), "20260101000002_second.up.sql", "SELECT 2;");
    write_file(dir.path(), "20260101000001_first.up.sql", "SELECT 1;");
    write_file(dir.path(), "20260101000001_first.down.sql", "DROP 1;");

    let result = scan(dir.path()).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].version, 20_260_101_000_001);
    assert_eq!(result[0].name, "first");
    assert_eq!(result[0].up_sql, "SELECT 1;");
    assert_eq!(result[0].down_sql.as_deref(), Some("DROP 1;"));
    assert_eq!(result[1].version, 20_260_101_000_002);
    assert_eq!(result[1].name, "second");
    assert!(result[1].down_sql.is_none());
}

#[test]
fn scan_produces_distinct_checksums_for_distinct_content() {
    let dir = tempdir().unwrap();
    write_file(dir.path(), "20260101000001_first.up.sql", "SELECT 1;");
    write_file(dir.path(), "20260101000002_second.up.sql", "SELECT 2;");

    let result = scan(dir.path()).unwrap();
    assert_ne!(result[0].checksum, result[1].checksum);
}

#[test]
fn scan_is_deterministic_for_same_content() {
    let dir = tempdir().unwrap();
    write_file(dir.path(), "20260101000001_first.up.sql", "SELECT 1;");

    let first = scan(dir.path()).unwrap();
    let second = scan(dir.path()).unwrap();
    assert_eq!(first[0].checksum, second[0].checksum);
}

#[test]
fn scan_rejects_duplicate_versions() {
    let dir = tempdir().unwrap();
    write_file(dir.path(), "20260101000001_first.up.sql", "SELECT 1;");
    write_file(dir.path(), "20260101000001_dup.up.sql", "SELECT 2;");

    let err = scan(dir.path()).unwrap_err();
    assert!(err.to_string().contains("duplicate migration version"));
}

#[test]
fn scan_rejects_invalid_filename() {
    let dir = tempdir().unwrap();
    write_file(dir.path(), "notanumber_first.up.sql", "SELECT 1;");

    let err = scan(dir.path()).unwrap_err();
    assert!(err.to_string().contains("invalid migration file name"));
}

#[test]
fn scan_ignores_unrelated_files() {
    let dir = tempdir().unwrap();
    write_file(dir.path(), "README.md", "# docs");
    write_file(dir.path(), "20260101000001_first.up.sql", "SELECT 1;");

    let result = scan(dir.path()).unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn scan_errors_when_directory_missing() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("does-not-exist");
    let err = scan(&missing).unwrap_err();
    assert!(err.to_string().contains("migration directory not found"));
}
