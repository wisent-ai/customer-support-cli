//! Run the built `customer-support` binary for the moment `--at` states and
//! for a ticket's update time, the way a caller does.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use chrono::{DateTime, Utc};
use serde_json::Value;

const POLICY: &str = r#"{"priorities":{
  "low":{"firstResponseHours":48,"resolutionHours":240},
  "normal":{"firstResponseHours":24,"resolutionHours":120},
  "high":{"firstResponseHours":8,"resolutionHours":48},
  "urgent":{"firstResponseHours":1,"resolutionHours":8}}}"#;
const TICKET: &str = r#"{"id":"T-moment","createdAt":"2026-01-02T03:04:05Z"}"#;
const UPDATED: &str = r#"{"id":"T-updated","createdAt":"2026-01-02T03:04:05Z","updatedAt":"2026-01-03T00:00:00+02:00"}"#;

/// A directory under the build's own scratch area for one case's inputs.
fn case(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("moment")
        .join(name);
    std::fs::create_dir_all(&directory).expect("create the case directory");
    directory
}

fn write(directory: &Path, file: &str, content: &str) -> PathBuf {
    let path = directory.join(file);
    std::fs::write(&path, content).expect("write a case input");
    path
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_customer-support-cli"))
        .args(arguments)
        .output()
        .expect("run customer-support")
}

fn json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("customer-support prints JSON")
}

fn sla(name: &str, at: &[&str]) -> Output {
    let directory = case(name);
    let ticket = write(&directory, "ticket.json", TICKET);
    let policy = write(&directory, "policy.json", POLICY);
    let mut arguments = vec![
        "sla",
        "--ticket",
        ticket.to_str().expect("UTF-8 path"),
        "--policy",
        policy.to_str().expect("UTF-8 path"),
    ];
    arguments.extend_from_slice(at);
    run(&arguments)
}

fn evaluated_at(output: &Output) -> DateTime<Utc> {
    let document = json(output);
    let stated = document["evaluatedAt"]
        .as_str()
        .expect("the SLA names the moment it was evaluated at");
    DateTime::parse_from_rfc3339(stated)
        .expect("evaluatedAt is RFC 3339")
        .with_timezone(&Utc)
}

#[test]
fn a_stated_moment_is_the_moment_evaluated_at() {
    let output = sla("stated", &["--at", "2026-02-03T04:05:06+01:00"]);
    let expected = DateTime::parse_from_rfc3339("2026-02-03T03:05:06Z").expect("a date");
    assert_eq!(evaluated_at(&output), expected);
}

#[test]
fn an_omitted_or_now_moment_is_the_time_the_command_reads_it() {
    for (name, at) in [("omitted", &[][..]), ("now", &["--at", "now"][..])] {
        let before = Utc::now();
        let output = sla(name, at);
        let after = Utc::now();
        let moment = evaluated_at(&output);
        assert!(before <= moment && moment <= after, "{name}: {moment}");
    }
}

#[test]
fn a_moment_that_is_neither_a_date_nor_now_is_refused_by_name() {
    let output = sla("refused", &["--at", "yesterday"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(output.stdout.is_empty(), "{stderr}");
    assert!(stderr.contains("--at"), "{stderr}");
    assert!(stderr.contains("yesterday"), "{stderr}");
    assert!(
        stderr.contains("the moment must be a valid ISO-8601 date"),
        "{stderr}"
    );
}

#[test]
fn help_declares_now_as_the_default_moment() {
    for command in ["sla", "queue"] {
        let output = run(&[command, "--help"]);
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{help}");
        assert!(help.contains("[default: now]"), "{command}: {help}");
    }
}

#[test]
fn a_ticket_without_an_update_time_is_not_given_one() {
    let directory = case("updated");
    let plain = write(&directory, "plain.json", TICKET);
    let updated = write(&directory, "updated.json", UPDATED);
    let document = json(&run(&[
        "normalize",
        "--ticket",
        plain.to_str().expect("UTF-8 path"),
    ]));
    assert!(document["updatedAt"].is_null(), "{document}");
    let document = json(&run(&[
        "normalize",
        "--ticket",
        updated.to_str().expect("UTF-8 path"),
    ]));
    assert_eq!(
        document["updatedAt"], "2026-01-02T22:00:00.000Z",
        "{document}"
    );
}
