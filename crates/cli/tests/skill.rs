use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Seat(PathBuf);

impl Seat {
    fn new() -> Self {
        let name = format!(
            "ensign-skill-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir(&path).expect("create temp root");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove temp root");
    }
}

fn run(seat: &Seat, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ensign"))
        .args(args)
        .env("ENSIGN_HOME", seat.path().join("data"))
        .env("HOME", seat.path().join("home"))
        .output()
        .expect("run ensign")
}

#[test]
fn list() {
    let seat = Seat::new();
    let output = run(&seat, &["skill", "list"]);
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("no managed skill"));
}

#[test]
fn help() {
    let seat = Seat::new();
    let output = run(&seat, &["skill", "--help"]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for deed in ["install", "upgrade", "status", "stage", "list", "uninstall"] {
        assert!(stdout.contains(deed), "{stdout}");
    }
}
