use super::{Nest, reap};
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn ceremony(nest: &Nest, verb: &str, extra: &[String]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ensign-api"));
    command
        .env_clear()
        .env("HOME", &nest.0)
        .env("USERPROFILE", &nest.0)
        .env("LOCALAPPDATA", &nest.0)
        .env("ENSIGN_HOME", &nest.0)
        .arg(verb)
        .arg(&nest.0)
        .arg("--artifact")
        .arg(format!("signing=file:{}", nest.path("sign.pem").display()));
    for spec in extra {
        command.arg("--artifact").arg(spec);
    }
    command
}

fn seal(nest: &Nest, extra: &[String]) -> Output {
    let mut specs = vec![format!("sudo=file:{}", nest.path("sudo").display())];
    specs.extend_from_slice(extra);
    ceremony(nest, "bootstrap", &specs)
        .output()
        .expect("bootstrap process")
}

fn spec(slug: &str, path: &Path) -> String {
    format!("key:{slug}=file:{}", path.display())
}

#[test]
fn minted() {
    let nest = Nest::new();
    let path = nest.path("ledger.key");
    let first = seal(&nest, &[spec("ledger", &path)]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let held = std::fs::read(&path).expect("key");
    assert_eq!(String::from_utf8_lossy(&held).trim().len(), 64, "key shape");
    assert!(!first.stderr.windows(held.len()).any(|part| part == held));
    assert!(!first.stdout.windows(held.len()).any(|part| part == held));
    let second = seal(&nest, &[spec("ledger", &path)]);
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(std::fs::read(&path).expect("key replay"), held);
    let plain = seal(&nest, &[]);
    assert!(plain.status.success(), "a keyless replay refused");
    assert_eq!(std::fs::read(&path).expect("key kept"), held);
}

#[test]
fn foreign() {
    let nest = Nest::new();
    let path = nest.path("ledger.key");
    std::fs::write(&path, b"not a key").expect("foreign");
    let out = seal(&nest, &[spec("ledger", &path)]);
    assert!(!out.status.success(), "a foreign destination was accepted");
    assert!(String::from_utf8_lossy(&out.stderr).contains("holds no live key"));
    assert_eq!(std::fs::read(&path).expect("kept"), b"not a key");
}

#[test]
fn malformed() {
    let nest = Nest::new();
    for name in ["key:", "key:Ledger", "key:led ger", "keys"] {
        let out = seal(
            &nest,
            &[format!("{name}=file:{}", nest.path("k").display())],
        );
        assert!(!out.status.success(), "{name} accepted");
        assert!(!nest.path("k").exists(), "{name} wrote a key");
    }
}

#[test]
fn forbidden() {
    let nest = Nest::new();
    let path = nest.path("ledger.key");
    let sealed = seal(&nest, &[spec("ledger", &path)]);
    assert!(
        sealed.status.success(),
        "{}",
        String::from_utf8_lossy(&sealed.stderr)
    );
    let child = ceremony(&nest, "serve", &[spec("ledger", &path)])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("serve process");
    let out = reap(child);
    assert!(!out.status.success(), "serve accepted a key artifact");
    let note = String::from_utf8_lossy(&out.stderr);
    assert!(
        note.contains("key artifact is forbidden at runtime"),
        "serve exited for the wrong reason: {note}"
    );
}
