use super::server::Server;
use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

pub(super) struct Fixture {
    root: tempfile::TempDir,
    pub(super) server: Server,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
            server: Server::new(),
        };
        std::fs::create_dir_all(fixture.home().join("state")).unwrap();
        fixture.credential(&fixture.server.url);
        fixture
    }

    pub(super) fn home(&self) -> PathBuf {
        self.root.path().join("client")
    }

    pub(super) fn credential(&self, base: &str) {
        std::fs::write(
            self.home().join("state/pat.json"),
            serde_json::to_vec(&json!({"schema":1,"base":base,"token":"private-token"})).unwrap(),
        )
        .unwrap();
    }

    pub(super) fn configure(&self, text: &str) {
        std::fs::write(self.home().join("config.toml"), text).unwrap();
    }

    pub(super) fn run(&self, args: &[&str]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ensign"))
            .env_clear()
            .env("HOME", self.root.path())
            .env("USERPROFILE", self.root.path())
            .env("LOCALAPPDATA", self.root.path())
            .env("ENSIGN_HOME", self.home())
            .env("ENSIGN_URL", &self.server.url)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"private-password\n")
            .unwrap();
        child.wait_with_output().unwrap()
    }

    pub(super) fn records(&self) -> Vec<Value> {
        let mut atoms = Vec::new();
        for entry in std::fs::read_dir(self.home().join("state/locus")).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name == "active.jsonl" || name.starts_with("sealed-") {
                let text = std::fs::read_to_string(path).unwrap();
                atoms.extend(
                    text.lines()
                        .map(|line| serde_json::from_str::<Value>(line).unwrap()),
                );
            }
        }
        atoms
    }
}
