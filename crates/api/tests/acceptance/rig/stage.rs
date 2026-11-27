#![allow(dead_code)]

#[path = "scratch.rs"]
mod scratch;

pub use scratch::Scratch;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

pub struct Stage {
    pub origin: String,
    pub base: String,
    pub sudo: String,
    pub nest: PathBuf,
    exe: PathBuf,
    child: Child,
    room: tempfile::TempDir,
    scratch: Option<Scratch>,
}

impl Stage {
    pub fn new(exe: &Path) -> Self {
        Self::with(exe, &[])
    }

    pub fn with(exe: &Path, slugs: &[&str]) -> Self {
        let scratch = Scratch::open();
        let store = scratch.as_ref().map(Scratch::url);
        let room = tempfile::tempdir().expect("stage room");
        let nest = room.path().to_path_buf();
        std::fs::write(
            nest.join("ensign.toml"),
            "[listen]\nhost = \"127.0.0.1\"\nport = 0\nprefix = \"\"\n\n[store]\nkind = \"file\"\npath = \"estate.db\"\n\n[cache]\nkind = \"memory\"\n",
        )
        .expect("stage config");
        let sudo = nest.join("sudo");
        let signing = format!("signing=file:{}", nest.join("sign.pem").display());
        let mut boot = launch(exe, &nest, store);
        boot.arg("bootstrap")
            .arg(&nest)
            .arg("--artifact")
            .arg(format!("sudo=file:{}", sudo.display()))
            .arg("--artifact")
            .arg(&signing);
        for slug in slugs {
            let held = nest.join(format!("{slug}.key"));
            boot.arg("--artifact")
                .arg(format!("key:{slug}=file:{}", held.display()));
        }
        let done = boot.output().expect("bootstrap process");
        assert!(
            done.status.success(),
            "{}",
            String::from_utf8_lossy(&done.stderr)
        );
        let token = std::fs::read_to_string(&sudo).expect("sudo custody");
        let mut child = launch(exe, &nest, store)
            .arg("serve")
            .arg(&nest)
            .arg("--artifact")
            .arg(&signing)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("serve process");
        let origin = listen(&mut child);
        let stage = Self {
            base: format!("{origin}/api"),
            origin,
            sudo: token.trim().to_string(),
            nest,
            exe: exe.to_path_buf(),
            child,
            room,
            scratch,
        };
        stage.ready();
        stage
    }

    pub fn key(&self, slug: &str) -> String {
        let held = std::fs::read_to_string(self.nest.join(format!("{slug}.key")));
        held.expect("key custody").trim().to_string()
    }

    pub fn again(&self, specs: &[String]) -> Output {
        let store = self.scratch.as_ref().map(Scratch::url);
        let mut boot = launch(&self.exe, &self.nest, store);
        boot.arg("bootstrap")
            .arg(&self.nest)
            .arg("--artifact")
            .arg(format!("sudo=file:{}", self.nest.join("sudo").display()))
            .arg("--artifact")
            .arg(format!(
                "signing=file:{}",
                self.nest.join("sign.pem").display()
            ));
        for spec in specs {
            boot.arg("--artifact").arg(spec);
        }
        boot.output().expect("bootstrap process")
    }

    pub fn crown(&self) -> Vec<(String, String)> {
        vec![("authorization".into(), format!("sudo {}", self.sudo))]
    }

    fn ready(&self) {
        let url = format!("{}/health", self.base);
        for _ in 0..120 {
            if ureq::get(&url).call().is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        panic!("timeout {url}");
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn listen(child: &mut Child) -> String {
    let pipe = child.stderr.take().expect("serve stderr");
    let mut lines = BufReader::new(pipe).lines();
    let mut said = Vec::new();
    while let Some(Ok(line)) = lines.next() {
        let seen: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
        if seen["role"] == "api"
            && let Some(at) = seen["endpoint"].as_str()
        {
            let at = at.to_string();
            std::thread::spawn(move || lines.for_each(drop));
            return at;
        }
        said.push(line);
    }
    let _ = child.wait();
    panic!("serve exited before listening: {}", said.join("\n"));
}

fn launch(exe: &Path, nest: &Path, store: Option<&str>) -> Command {
    let mut command = Command::new(exe);
    command
        .env_clear()
        .env("HOME", nest)
        .env("USERPROFILE", nest)
        .env("LOCALAPPDATA", nest)
        .env("ENSIGN_HOME", nest);
    if let Some(url) = store {
        command
            .env("ENSIGN_STORE_KIND", "pg")
            .env("ENSIGN_STORE_URL", url);
    }
    command
}
