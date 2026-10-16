//! Run the built `customer-support` binary for the moment `--at` states and
//! for a ticket's update time, the way a caller does.
//!
//! With `CUSTOMER_SUPPORT_JOURNEY_REPORT=<file>` the passed journey also
//! writes a report there: the committed source revision it ran (a tree with
//! uncommitted changes is refused), the digests of the binary and of this
//! file, every command with its arguments, exit status and stderr, and the
//! final state it observed. The register entry the report answers is named by
//! `CUSTOMER_SUPPORT_JOURNEY_SUBJECT` and `CUSTOMER_SUPPORT_JOURNEY_CRITERION`;
//! each is refused by name when the report is asked for without it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const POLICY: &str = r#"{"priorities":{
  "low":{"firstResponseHours":48,"resolutionHours":240},
  "normal":{"firstResponseHours":24,"resolutionHours":120},
  "high":{"firstResponseHours":8,"resolutionHours":48},
  "urgent":{"firstResponseHours":1,"resolutionHours":8}}}"#;
const TICKET: &str = r#"{"id":"T-moment","createdAt":"2026-01-02T03:04:05Z"}"#;
const UPDATED: &str = r#"{"id":"T-updated","createdAt":"2026-01-02T03:04:05Z","updatedAt":"2026-01-03T00:00:00+02:00"}"#;
const BINARY: &str = env!("CARGO_BIN_EXE_customer-support-cli");

/// The commands one journey ran and what it saw, for its report.
#[derive(Default)]
struct Journey {
    commands: Vec<Value>,
    observed: Map<String, Value>,
}

impl Journey {
    /// Run the binary; a command that succeeds is recorded with its exit
    /// status, one that is refused is kept by the caller as an observation.
    fn run(&mut self, arguments: &[&str]) -> Output {
        let output = Command::new(BINARY)
            .args(arguments)
            .output()
            .expect("run customer-support");
        if output.status.success() {
            self.commands.push(json!({
                "executable": BINARY,
                "args": arguments,
                "exit": output.status.code(),
                "stderr": String::from_utf8_lossy(&output.stderr),
            }));
        }
        output
    }

    fn observe(&mut self, name: &str, value: Value) {
        self.observed.insert(name.to_owned(), value);
    }

    /// The command's help shows `now` as the default of `--at`.
    fn help_declares_now(&mut self, command: &str) {
        let output = self.run(&[command, "--help"]);
        let help = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(output.status.success(), "{help}");
        assert!(help.contains("[default: now]"), "{command}: {help}");
        self.observe(&format!("{command}Help"), json!(help));
    }
}

/// A directory under the build's own scratch area for the journey's inputs.
fn inputs() -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("moment");
    std::fs::create_dir_all(&directory).expect("create the input directory");
    directory
}

fn write(directory: &Path, file: &str, content: &str) -> String {
    let path = directory.join(file);
    std::fs::write(&path, content).expect("write a journey input");
    path.to_str().expect("UTF-8 path").to_owned()
}

fn json_of(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("customer-support prints JSON")
}

fn evaluated_at(output: &Output) -> DateTime<Utc> {
    let document = json_of(output);
    let stated = document["evaluatedAt"]
        .as_str()
        .expect("the SLA names the moment it was evaluated at");
    DateTime::parse_from_rfc3339(stated)
        .expect("evaluatedAt is RFC 3339")
        .with_timezone(&Utc)
}

fn git(arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .args(arguments)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn named(variable: &str) -> String {
    match std::env::var(variable) {
        Ok(value) => value,
        Err(error) => panic!(
            "{variable} names what the report answers and CUSTOMER_SUPPORT_JOURNEY_REPORT asks for a report: {error}"
        ),
    }
}

/// Write the report a passed journey was asked for, refusing an uncommitted
/// tree.
fn report(journey: Journey) {
    let Ok(path) = std::env::var("CUSTOMER_SUPPORT_JOURNEY_REPORT") else {
        return;
    };
    let changed = git(&["status", "--porcelain"]);
    assert!(
        changed.is_empty(),
        "a report names a committed source; the tree has changes:\n{changed}"
    );
    let binary = std::fs::read(BINARY).expect("read the binary");
    let document = json!({
        "defectId": named("CUSTOMER_SUPPORT_JOURNEY_SUBJECT"),
        "criterion": named("CUSTOMER_SUPPORT_JOURNEY_CRITERION"),
        "sourceRevision": git(&["rev-parse", "HEAD"]),
        "result": "passed",
        "binarySha256": format!("{:x}", Sha256::digest(binary)),
        "testSha256": format!("{:x}", Sha256::digest(include_bytes!("at.rs"))),
        "commands": journey.commands,
        "observedFinalState": journey.observed,
    });
    let bytes = serde_json::to_vec_pretty(&document).expect("serialize the report");
    if let Err(error) = std::fs::write(&path, bytes) {
        panic!("write the report {path}: {error}");
    }
}

#[test]
fn the_moment_and_the_update_time_are_what_the_caller_states() {
    let mut journey = Journey::default();
    let directory = inputs();
    let ticket = write(&directory, "ticket.json", TICKET);
    let updated = write(&directory, "updated.json", UPDATED);
    let policy = write(&directory, "policy.json", POLICY);
    let sla = ["sla", "--ticket", &ticket, "--policy", &policy];

    let stated = journey.run(&[&sla[..], &["--at", "2026-02-03T04:05:06+01:00"]].concat());
    let moment = evaluated_at(&stated);
    let expected = DateTime::parse_from_rfc3339("2026-02-03T03:05:06Z").expect("a date");
    assert_eq!(moment, expected);
    journey.observe("statedMoment", json!(moment.to_rfc3339()));

    for (name, at) in [
        ("omittedMoment", &[][..]),
        ("nowMoment", &["--at", "now"][..]),
    ] {
        let before = Utc::now();
        let output = journey.run(&[&sla[..], at].concat());
        let after = Utc::now();
        let moment = evaluated_at(&output);
        assert!(before <= moment && moment <= after, "{name}: {moment}");
        journey.observe(
            name,
            json!({"before": before.to_rfc3339(), "evaluatedAt": moment.to_rfc3339(), "after": after.to_rfc3339()}),
        );
    }

    let refused = journey.run(&[&sla[..], &["--at", "yesterday"]].concat());
    let stderr = String::from_utf8_lossy(&refused.stderr).into_owned();
    assert!(!refused.status.success(), "{stderr}");
    assert!(refused.stdout.is_empty(), "{stderr}");
    assert!(stderr.contains("--at"), "{stderr}");
    assert!(stderr.contains("yesterday"), "{stderr}");
    assert!(
        stderr.contains("the moment must be a valid ISO-8601 date"),
        "{stderr}"
    );
    journey.observe(
        "refusedMoment",
        json!({"exit": refused.status.code(), "stderr": stderr}),
    );

    journey.help_declares_now("sla");
    journey.help_declares_now("queue");

    let plain = json_of(&journey.run(&["normalize", "--ticket", &ticket]));
    assert!(plain["updatedAt"].is_null(), "{plain}");
    let stated = json_of(&journey.run(&["normalize", "--ticket", &updated]));
    assert_eq!(stated["updatedAt"], "2026-01-02T22:00:00.000Z", "{stated}");
    journey.observe(
        "updatedAt",
        json!({"absent": plain["updatedAt"], "stated": stated["updatedAt"]}),
    );

    report(journey);
}
