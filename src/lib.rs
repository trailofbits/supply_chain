use anyhow::{Context, Result, anyhow, bail, ensure};
use similar_asserts::SimpleDiff;
use std::{
    env,
    fs::{read_to_string, write},
    path::Path,
    process::{Command, ExitStatus},
    str::FromStr,
    sync::OnceLock,
};

/// Checks or updates a JSON snapshot of a normalized supply-chain report.
///
/// On the first call in each process, the function runs
/// `cargo supply-chain update --cache-max-age=0s`. The exit status of the update command is
/// ignored.
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
    check_impl(path, enabled("BLESS")).unwrap()
}

fn check_impl(path: impl AsRef<Path>, bless: bool) -> Result<()> {
    update()?;

    let report_actual = generate_report(".")?;

    if bless {
        write(&path, report_actual)
            .with_context(|| format!("failed to write `{}`", path.as_ref().display()))?;
    } else {
        let report_expected = read_to_string(&path)
            .with_context(|| format!("failed to read `{}`", path.as_ref().display()))?;

        ensure!(
            report_expected == report_actual,
            "{}",
            SimpleDiff::from_str(&report_expected, &report_actual, "expected", "actual")
        );
    }

    Ok(())
}

fn update() -> Result<()> {
    static UPDATED: OnceLock<Result<()>> = OnceLock::new();

    let result = UPDATED.get_or_init(|| {
        let mut command = Command::new("cargo");
        command.args(["supply-chain", "update", "--cache-max-age=0s"]);
        let _: ExitStatus = command
            .status()
            .with_context(|| format!("failed to get status of command: {command:?}"))?;
        Ok(())
    });

    result
        .as_ref()
        .copied()
        .map_err(|error| anyhow!("{error:#}"))
}

fn generate_report(dir: impl AsRef<Path>) -> Result<String> {
    let mut command = Command::new("cargo");
    command
        .args(["supply-chain", "json", "--no-dev"])
        .current_dir(dir);
    let output = command
        .output()
        .with_context(|| format!("failed to get output of command: {command:?}"))?;
    if !output.status.success() {
        bail!(
            "command failed: {command:?}\n{}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
    }

    normalize_report(&output.stdout)
}

fn normalize_report(report_bytes: &[u8]) -> Result<String> {
    let report = std::str::from_utf8(report_bytes)?;
    let mut value = serde_json::Value::from_str(report)?;
    remove_avatars(&mut value);
    serde_json::to_string_pretty(&value).map_err(Into::into)
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

#[cfg(test)]
mod tests {
    use super::{check_impl, generate_report, normalize_report, remove_avatars};
    use serde_json::{json, to_string_pretty};
    use std::fs;

    #[cfg_attr(
        dylint_lib = "supplementary",
        allow(
            non_thread_safe_call_in_test,
            reason = "the snapshot is written to a temporary directory"
        )
    )]
    #[test]
    fn check_impl_writes_and_checks_snapshot() {
        let tempdir = tempfile::tempdir().unwrap();
        let path_buf = tempdir.path().join("supply_chain.json");

        check_impl(&path_buf, true).unwrap();
        assert!(path_buf.is_file());

        check_impl(&path_buf, false).unwrap();
    }

    #[test]
    fn check_impl_reports_missing_snapshot() {
        let tempdir = tempfile::tempdir().unwrap();
        let path_buf = tempdir.path().join("missing.json");

        let error = check_impl(&path_buf, false).unwrap_err();

        assert_eq!(
            format!("failed to read `{}`", path_buf.display()),
            error.to_string()
        );
    }

    #[cfg_attr(
        dylint_lib = "supplementary",
        allow(
            non_thread_safe_call_in_test,
            reason = "the snapshot is written to a temporary directory"
        )
    )]
    #[test]
    fn check_impl_reports_mismatched_snapshot() {
        let tempdir = tempfile::tempdir().unwrap();
        let path_buf = tempdir.path().join("supply_chain.json");
        fs::write(&path_buf, "{}").unwrap();

        let error = check_impl(&path_buf, false).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("expected"));
        assert!(message.contains("actual"));
    }

    #[test]
    fn generate_report_includes_stderr_when_command_fails() {
        let tempdir = tempfile::tempdir().unwrap();

        let error = generate_report(&tempdir).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("command failed"), "{message}");
        assert!(message.contains("could not find `Cargo.toml`"), "{message}");
    }

    #[test]
    fn normalize_report_removes_avatars_and_pretty_prints() {
        let report_bytes = br#"{
"crates_io_crates":{"example-crate":[{
"id":1001,"kind":"user","login":"alice-example","name":"Alice Example","avatar":"ignored"
}]},
"not_audited":{"foreign_crates":[],"local_crates":["example-project"]}
}"#;
        let expected = json!({
            "crates_io_crates": {
                "example-crate": [{
                    "id": 1001,
                    "kind": "user",
                    "login": "alice-example",
                    "name": "Alice Example"
                }]
            },
            "not_audited": {
                "foreign_crates": [],
                "local_crates": ["example-project"]
            }
        });

        assert_eq!(
            to_string_pretty(&expected).unwrap(),
            normalize_report(report_bytes).unwrap()
        );
    }

    #[test]
    fn normalize_report_rejects_invalid_utf8() {
        assert!(normalize_report(&[0xff]).is_err());
    }

    #[test]
    fn normalize_report_rejects_invalid_json() {
        assert!(normalize_report(b"not JSON").is_err());
    }

    #[test]
    fn remove_avatars_removes_nested_avatar_fields() {
        let mut value = json!({
            "crates_io_crates": {
                "example-crate": [
                    {
                        "avatar": "ignored",
                        "id": 1001,
                        "kind": "user",
                        "login": "alice-example",
                        "name": "Alice Example"
                    },
                    {
                        "avatar": null,
                        "id": 1002,
                        "kind": "team",
                        "login": "github:example-org:publishers",
                        "name": "Example Publishers"
                    }
                ]
            }
        });

        remove_avatars(&mut value);

        assert_eq!(
            json!({
                "crates_io_crates": {
                    "example-crate": [
                        {
                            "id": 1001,
                            "kind": "user",
                            "login": "alice-example",
                            "name": "Alice Example"
                        },
                        {
                            "id": 1002,
                            "kind": "team",
                            "login": "github:example-org:publishers",
                            "name": "Example Publishers"
                        }
                    ]
                }
            }),
            value
        );
    }

    #[test]
    fn remove_avatars_preserves_non_avatar_values() {
        let mut value = json!({
            "crates_io_crates": {
                "example-crate": [{
                    "id": 1001,
                    "kind": "user",
                    "login": "alice-example",
                    "name": "Alice Example"
                }]
            },
            "not_audited": {
                "foreign_crates": [],
                "local_crates": ["example-project"]
            }
        });
        let expected = value.clone();

        remove_avatars(&mut value);

        assert_eq!(expected, value);
    }
}
