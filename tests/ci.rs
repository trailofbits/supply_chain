use assert_cmd::assert::OutputAssertExt;
use regex::Regex;
use std::{fs::read_to_string, process::Command};
use tempfile::tempdir;

#[test]
fn clippy() {
    let mut command = assert_cmd::Command::new("cargo");
    command.args([
        "+nightly",
        "clippy",
        "--all-targets",
        "--offline",
        "--",
        "--deny=warnings",
    ]);
    command.assert().success();
}

#[test]
fn dylint() {
    let assert = Command::new("cargo")
        .args(["dylint", "--all", "--", "--all-targets"])
        .env("DYLINT_RUSTFLAGS", "--deny warnings")
        .assert();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(assert.try_success().is_ok(), "{}", stderr);
}

#[cfg_attr(target_os = "windows", ignore = "`markdown_link_check` not installed")]
#[test]
fn markdown_link_check() {
    let tempdir = tempdir().unwrap();

    Command::new("npm")
        .args(["install", "markdown-link-check"])
        .current_dir(&tempdir)
        .assert()
        .success();

    let readme_md = concat!(env!("CARGO_MANIFEST_DIR"), "/README.md");

    Command::new("npx")
        .args(["markdown-link-check", readme_md])
        .current_dir(&tempdir)
        .assert()
        .success();
}

#[test]
fn readme_reference_links_are_sorted() {
    let re = Regex::new(r"^\[[^^\]]*\]:").unwrap();
    let readme = read_to_string("README.md").unwrap();
    let links = readme
        .lines()
        .filter(|line| re.is_match(line))
        .collect::<Vec<_>>();
    let mut links_sorted = links.clone();
    links_sorted.sort_unstable();
    assert!(
        links_sorted == links,
        "contents of README.md are not what was expected:\n{}",
        links_sorted.join("\n")
    );
}
