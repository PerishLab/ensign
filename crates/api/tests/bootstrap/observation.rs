use super::Nest;
use serde_json::Value;
use std::process::{Command, Output};
const ENABLED: &str = "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n";
fn run(nest: &Nest, seat: &Nest, invalid: bool) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ensign-api"))
        .env_clear()
        .env("ENSIGN_HOME", &seat.0)
        .env("HOME", &seat.0)
        .env("USERPROFILE", &seat.0)
        .env("LOCALAPPDATA", &seat.0)
        .arg("bootstrap")
        .arg(&nest.0)
        .arg("--artifact")
        .arg(format!("sudo=file:{}", nest.path("sudo").display()))
        .arg("--artifact")
        .arg(if invalid {
            "signing=private-invalid".into()
        } else {
            format!("signing=file:{}", nest.path("sign.pem").display())
        })
        .output()
        .unwrap()
}
fn configure(seat: &Nest, text: &str) {
    std::fs::write(seat.path("config.toml"), text).unwrap();
}
fn records(seat: &Nest) -> Vec<Value> {
    let mut atoms = Vec::new();
    for entry in std::fs::read_dir(seat.path("state/locus")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if name == "active.jsonl" || name.starts_with("sealed-") {
            atoms.extend(
                std::fs::read_to_string(path)
                    .unwrap()
                    .lines()
                    .map(|line| serde_json::from_str::<Value>(line).unwrap()),
            );
        }
    }
    atoms
}
#[test]
fn buffered() {
    let nest = Nest::new();
    let seat = Nest::new();
    configure(&seat, ENABLED);
    assert_eq!(run(&nest, &seat, false).status.code(), Some(0));
    let sudo = std::fs::read(nest.path("sudo")).unwrap();
    let signing = std::fs::read(nest.path("sign.pem")).unwrap();
    assert_eq!(run(&nest, &seat, false).status.code(), Some(0));
    assert_eq!(run(&nest, &seat, true).status.code(), Some(1));
    assert_eq!(std::fs::read(nest.path("sudo")).unwrap(), sudo);
    assert_eq!(std::fs::read(nest.path("sign.pem")).unwrap(), signing);
    let atoms = records(&seat);
    assert!(atoms.iter().all(|a| a["producer"] == "ensign"));
    assert!(
        atoms
            .iter()
            .any(|a| a["context"]["ensign.command"] == "api.bootstrap")
    );
    assert_eq!(
        atoms
            .iter()
            .filter(|a| a["source"]["edge"] == "enter")
            .count(),
        3
    );
    assert_eq!(
        atoms
            .iter()
            .filter(|a| a["source"]["edge"] == "return")
            .count(),
        2
    );
    assert_eq!(
        atoms
            .iter()
            .filter(|a| a["payload"]["event"] == "cli.finish")
            .count(),
        2
    );
    let text = serde_json::to_string(&atoms).unwrap();
    assert!(!text.contains("private-invalid"));
    assert!(!text.contains(nest.0.to_str().unwrap()));
    assert!(!text.contains(std::str::from_utf8(&sudo).unwrap()));
    assert!(!text.contains(std::str::from_utf8(&signing).unwrap()));
}
#[test]
fn isolated() {
    let nest = Nest::new();
    let seat = Nest::new();
    let baseline = run(&nest, &seat, false);
    assert_eq!(baseline.status.code(), Some(0));
    let sudo = std::fs::read(nest.path("sudo")).unwrap();
    let signing = std::fs::read(nest.path("sign.pem")).unwrap();
    configure(&seat, "[locus]\nenabled = false\n");
    let off = run(&nest, &seat, false);
    assert_eq!(
        (off.status.code(), off.stdout, off.stderr),
        (
            baseline.status.code(),
            baseline.stdout.clone(),
            baseline.stderr.clone()
        )
    );
    assert!(!seat.path("state/locus").exists());
    for text in [
        ENABLED,
        "[locus]\nenabled = 'private-setting'\n",
        "[locus]\nenabled = true\nendpoint = ''\n",
    ] {
        configure(&seat, text);
        let out = run(&nest, &seat, false);
        assert_eq!(out.status.code(), baseline.status.code());
        assert_eq!(out.stdout, baseline.stdout);
        assert!(!out.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("private-setting"));
        assert_eq!(std::fs::read(nest.path("sudo")).unwrap(), sudo);
        assert_eq!(std::fs::read(nest.path("sign.pem")).unwrap(), signing);
    }
    let blocked = Nest::new();
    configure(&blocked, ENABLED);
    std::fs::write(blocked.path("state"), "occupied").unwrap();
    let out = run(&nest, &blocked, false);
    assert_eq!(out.status.code(), baseline.status.code());
    assert_eq!(out.stdout, baseline.stdout);
    assert!(!out.stderr.is_empty());
}
