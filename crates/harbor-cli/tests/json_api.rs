use serde_json::Value;
use std::fs;
use std::process::{Command, Output};

fn request(home: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_harbor"))
        .env("HOME", home)
        .arg("--json")
        .args(args)
        .output()
        .unwrap()
}
fn response(output: &Output) -> Value {
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["api_version"], 1);
    value
}

#[test]
fn empty_list_has_versioned_response_and_no_human_output() {
    let temp = tempfile::tempdir().unwrap();
    let output = request(temp.path(), &["list"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = response(&output);
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["profiles"], serde_json::json!([]));
}

#[test]
fn missing_profile_and_unsupported_command_return_json_errors() {
    let temp = tempfile::tempdir().unwrap();
    for args in [
        ["show", "missing"],
        ["logs", "missing"],
        ["start", "../bad"],
    ] {
        let output = request(temp.path(), &args);
        assert!(!output.status.success());
        let value = response(&output);
        assert_eq!(value["ok"], false);
        assert!(value["error"].as_str().unwrap().len() > 5);
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn json_mode_does_not_bypass_default_account_guard() {
    let temp = tempfile::tempdir().unwrap();
    let protected = temp.path().join(".codex");
    fs::create_dir(&protected).unwrap();
    fs::write(protected.join("sentinel"), b"test-only").unwrap();
    let output = request(
        temp.path(),
        &["--root", protected.to_str().unwrap(), "list"],
    );
    assert!(!output.status.success());
    assert_eq!(response(&output)["ok"], false);
    assert_eq!(fs::read_dir(&protected).unwrap().count(), 1);
}

#[test]
fn failed_doctor_keeps_report_in_structured_result() {
    let temp = tempfile::tempdir().unwrap();
    let output = request(temp.path(), &["doctor", "missing"]);
    assert!(output.status.success());
    let value = response(&output);
    assert_eq!(value["data"]["passed"], false);
    assert!(value["data"]["error"].is_string());
}
