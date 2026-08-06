use assert_cmd::assert::OutputAssertExt;
use serde_json::{Value, from_str};
use std::{fs::read_to_string, path::Path, process::Command};

const FIXTURE: &str = "fixtures/cargo_10801";

const BEHAVIOR_CHANGED: &str =
    "suggesting the behavior described in Cargo issue #10801 has changed";

#[test]
fn cargo_10801() {
    Command::new("cargo")
        .args(["test", "--locked"])
        .env_remove("CARGO_TERM_COLOR")
        .current_dir(FIXTURE)
        .assert()
        .success();

    // The nested test verifies only that the fixture's report matches its snapshot. Verify that
    // the snapshot still exhibits the behavior described in the "Known issues" section of
    // README.md: `time` is reported even though no feature enables it, while `chrono`, which no
    // feature mentions, is not.
    let path_buf = Path::new(FIXTURE).join("supply_chain.json");
    let snapshot = read_to_string(&path_buf).unwrap();
    let value = from_str::<Value>(&snapshot).unwrap();
    let crates = value["crates_io_crates"].as_object().unwrap();

    assert!(
        crates.contains_key("time"),
        "`time` is missing from `{}`, {BEHAVIOR_CHANGED}",
        path_buf.display()
    );
    assert!(
        !crates.contains_key("chrono"),
        "`chrono` appears in `{}`, {BEHAVIOR_CHANGED}",
        path_buf.display()
    );
}
