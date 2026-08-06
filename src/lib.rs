use assert_cmd::assert::OutputAssertExt;
use similar_asserts::SimpleDiff;
use std::{
    env,
    fs::{read_to_string, write},
    path::Path,
    process::{Command, ExitStatus},
    str::FromStr,
};

/// Checks or updates a JSON snapshot of a normalized supply-chain report.
///
/// The function first runs `cargo supply-chain update --cache-max-age=0s`. The exit status of the
/// update command is ignored.
///
/// The function then runs `cargo supply-chain json --no-dev`. The report is normalized by removing
/// all `avatar` fields and pretty-printing the JSON.
///
/// If `BLESS` is set to a value other than `"0"`, the normalized report is written to `path`.
/// Otherwise, the report is compared to the snapshot at `path`. The function panics if they
/// differ.
///
/// # Panics
///
/// Panics if a command cannot be started, the report command fails, its output is not valid UTF-8
/// or JSON, the snapshot cannot be read or written, or the normalized report differs from the
/// snapshot.
pub fn check(path: impl AsRef<Path>) {
    let mut command = Command::new("cargo");
    command.args(["supply-chain", "update", "--cache-max-age=0s"]);
    let _: ExitStatus = command.status().unwrap();

    let mut command = Command::new("cargo");
    command.args(["supply-chain", "json", "--no-dev"]);
    let assert = command.assert().success();

    let stdout_actual = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let mut value = serde_json::Value::from_str(stdout_actual).unwrap();
    remove_avatars(&mut value);
    let stdout_normalized = serde_json::to_string_pretty(&value).unwrap();

    if enabled("BLESS") {
        write(path, stdout_normalized).unwrap();
    } else {
        let stdout_expected = read_to_string(path).unwrap();

        assert!(
            stdout_expected == stdout_normalized,
            "{}",
            SimpleDiff::from_str(&stdout_expected, &stdout_normalized, "left", "right")
        );
    }
}

fn remove_avatars(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
        serde_json::Value::Array(array) => {
            for value in array {
                remove_avatars(value);
            }
        }
        serde_json::Value::Object(object) => {
            object.retain(|key, value| {
                if key == "avatar" {
                    return false;
                }
                remove_avatars(value);
                true
            });
        }
    }
}

fn enabled(key: &str) -> bool {
    env::var_os(key).is_some_and(|value| value != "0")
}
