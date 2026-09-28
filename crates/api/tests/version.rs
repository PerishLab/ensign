use std::process::Command;

#[test]
fn stamp() {
    let output = Command::new(env!("CARGO_BIN_EXE_ensign-api"))
        .arg("--version")
        .output()
        .expect("version process");
    assert!(output.status.success());
    let line = String::from_utf8(output.stdout).expect("utf8");
    let marker = line
        .trim_end()
        .strip_prefix("ensign-api v")
        .expect("binary name");
    assert!(!marker.is_empty() && !marker.contains(' '), "{line:?}");
}
