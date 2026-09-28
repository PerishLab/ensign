use serde_json::Value;
use std::process::{Command, Output};

const CODES: &[&str] = &["bootstrap.sudo-custody", "signing.artifact-invalid"];

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ensign"))
        .args(args)
        .output()
        .expect("ensign")
}

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("utf8")
}

fn json(args: &[&str]) -> Value {
    let output = run(args);
    assert!(output.status.success(), "{}", text(output.stderr));
    serde_json::from_slice(&output.stdout).expect("json")
}

#[test]
fn parity() {
    for code in CODES {
        let output = run(&["cookbook", code]);
        assert!(output.status.success());
        let human = text(output.stdout);
        let value = json(&["cookbook", code, "--json"]);
        assert_eq!(value["code"], *code);
        for (heading, field) in [
            ("Trigger", "trigger"),
            ("Solution", "solution"),
            ("Evidence", "evidence"),
            ("EXIT", "exit"),
        ] {
            assert!(human.contains(&format!("## {heading}")), "{human}");
            assert!(
                human.contains(value[field].as_str().expect(field)),
                "{human}"
            );
        }
    }
}

#[test]
fn ordered() {
    let value = json(&["cookbook", "--json"]);
    let codes = value["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|entry| entry["code"].as_str().expect("code"))
        .collect::<Vec<_>>();
    assert_eq!(codes, CODES);
}

#[test]
fn unknown() {
    let output = run(&["cookbook", "sudo"]);
    assert!(!output.status.success());
    let stderr = text(output.stderr);
    assert!(stderr.contains("unknown Cookbook code `sudo`"), "{stderr}");
    assert!(stderr.contains(&CODES.join(", ")), "{stderr}");
}

#[test]
fn simple() {
    let output = run(&["login"]);
    assert!(!output.status.success());
    let stderr = text(output.stderr);
    assert!(stderr.contains("login takes a login name"), "{stderr}");
    assert!(!stderr.contains("cookbook"), "{stderr}");
}
