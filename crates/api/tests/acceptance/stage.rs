#![allow(dead_code)]

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

struct Unit;

static SERIAL: Mutex<Unit> = Mutex::new(Unit);

pub struct Stage {
    pub origin: String,
    pub base: String,
    pub sudo: String,
    pub nest: PathBuf,
    exe: PathBuf,
    child: Child,
    room: tempfile::TempDir,
    hold: Option<MutexGuard<'static, Unit>>,
}

impl Stage {
    pub fn new(exe: &Path) -> Self {
        Self::with(exe, &[])
    }

    pub fn with(exe: &Path, slugs: &[&str]) -> Self {
        let store = std::env::var("API_STORE_URL")
            .ok()
            .filter(|url| !url.is_empty());
        let hold = store
            .as_ref()
            .map(|_| SERIAL.lock().unwrap_or_else(|poison| poison.into_inner()));
        let room = tempfile::tempdir().expect("stage room");
        let nest = room.path().to_path_buf();
        let port = TcpListener::bind("127.0.0.1:0")
            .and_then(|probe| probe.local_addr())
            .expect("free port")
            .port();
        std::fs::write(
            nest.join("ensign.toml"),
            format!(
                "[listen]\nhost = \"127.0.0.1\"\nport = {port}\nprefix = \"\"\n\n[store]\nkind = \"file\"\npath = \"estate.db\"\n\n[cache]\nkind = \"memory\"\n"
            ),
        )
        .expect("stage config");
        let sudo = nest.join("sudo");
        let signing = format!("signing=file:{}", nest.join("sign.pem").display());
        let mut boot = launch(exe, &nest, store.as_deref(), true);
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
        let child = launch(exe, &nest, store.as_deref(), false)
            .arg("serve")
            .arg(&nest)
            .arg("--artifact")
            .arg(&signing)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("serve process");
        let origin = format!("http://127.0.0.1:{port}");
        let stage = Self {
            base: format!("{origin}/api"),
            origin,
            sudo: token.trim().to_string(),
            nest,
            exe: exe.to_path_buf(),
            child,
            room,
            hold,
        };
        stage.ready();
        stage
    }

    pub fn key(&self, slug: &str) -> String {
        let held = std::fs::read_to_string(self.nest.join(format!("{slug}.key")));
        held.expect("key custody").trim().to_string()
    }

    pub fn again(&self, specs: &[String]) -> Output {
        let store = std::env::var("API_STORE_URL")
            .ok()
            .filter(|url| !url.is_empty());
        let mut boot = launch(&self.exe, &self.nest, store.as_deref(), false);
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

fn launch(exe: &Path, nest: &Path, store: Option<&str>, fresh: bool) -> Command {
    let mut command = Command::new(exe);
    command
        .env_clear()
        .env("HOME", nest)
        .env("USERPROFILE", nest)
        .env("LOCALAPPDATA", nest)
        .env("ENSIGN_HOME", nest);
    if let Some(url) = store {
        command
            .env("API_STORE_KIND", "pg")
            .env("API_STORE_URL", url);
        if fresh {
            command.env("API_FRESH", "true");
        }
    }
    command
}
