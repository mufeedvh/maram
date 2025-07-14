//! Integration tests for maram

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

/// Helper to create a test directory structure
fn create_test_tree() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // Create directory structure
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("tests")).unwrap();
    fs::create_dir(root.join("docs")).unwrap();
    fs::create_dir(root.join(".git")).unwrap();
    
    // Create files with content
    let mut file = File::create(root.join("README.md")).unwrap();
    writeln!(file, "# Test Project\n\nThis is a test.").unwrap();
    
    let mut file = File::create(root.join("Cargo.toml")).unwrap();
    writeln!(file, "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();
    
    let mut file = File::create(root.join("src/main.rs")).unwrap();
    writeln!(file, "fn main() {{\n    println!(\"Hello, world!\");\n}}").unwrap();
    
    let mut file = File::create(root.join("src/lib.rs")).unwrap();
    writeln!(file, "pub fn add(a: i32, b: i32) -> i32 {{\n    a + b\n}}").unwrap();
    
    let mut file = File::create(root.join("tests/test.rs")).unwrap();
    writeln!(file, "#[test]\nfn test_add() {{\n    assert_eq!(2 + 2, 4);\n}}").unwrap();
    
    let mut file = File::create(root.join(".gitignore")).unwrap();
    writeln!(file, "target/\n*.log").unwrap();
    
    temp_dir
}

#[test]
fn test_basic_tree() {
    let temp_dir = create_test_tree();
    
    let mut cmd = Command::cargo_bin("maram").unwrap();
    cmd.arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("Cargo.toml"))
        .stdout(predicate::str::contains("src"));
}

#[test]
fn test_show_hidden() {
    let temp_dir = create_test_tree();
    
    // Without --all, .git should not be shown
    let mut cmd = Command::cargo_bin("maram").unwrap();
    cmd.arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(".git").not());
    
    // With --all, .git should be shown
    let mut cmd = Command::cargo_bin("maram").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--all")
        .assert()
        .success()
        .stdout(predicate::str::contains(".git"));
}

#[test]
fn test_max_depth() {
    let temp_dir = create_test_tree();
    
    let mut cmd = Command::cargo_bin("maram").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--depth=1")
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs").not())
        .stdout(predicate::str::contains("src"));
}

#[test]
fn test_only_dirs() {
    let temp_dir = create_test_tree();
    
    let mut cmd = Command::cargo_bin("maram").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--only-dirs")
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md").not())
        .stdout(predicate::str::contains("src"));
}

#[test]
fn test_json_output() {
    let temp_dir = create_test_tree();
    
    let mut cmd = Command::cargo_bin("maram").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--output=json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"size\""))
        .stdout(predicate::str::contains("\"is_dir\""));
}

#[test]
fn test_sort_by_size() {
    let temp_dir = create_test_tree();
    
    // Create files with different sizes
    let mut file = File::create(temp_dir.path().join("small.txt")).unwrap();
    write!(file, "a").unwrap();
    
    let mut file = File::create(temp_dir.path().join("large.txt")).unwrap();
    write!(file, "{}", "x".repeat(1000)).unwrap();
    
    let mut cmd = Command::cargo_bin("maram").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--sort=size")
        .arg("--reverse")
        .assert()
        .success();
}

#[test]
fn test_search() {
    let temp_dir = create_test_tree();
    
    let mut cmd = Command::cargo_bin("maram").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--search=main")
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("lib.rs").not());
}

#[test]
fn test_gitignore() {
    let temp_dir = create_test_tree();
    
    // Create a file that should be ignored
    File::create(temp_dir.path().join("test.log")).unwrap();
    
    let mut cmd = Command::cargo_bin("maram").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--gitignore")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.log").not());
}