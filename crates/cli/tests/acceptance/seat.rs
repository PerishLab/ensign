use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

pub struct Run {
    pub code: i32,
    pub out: String,
    pub err: String,
}

pub fn api() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let exe = Path::new(env!("CARGO_BIN_EXE_ensign"));
        let name = format!("ensign-api{}", std::env::consts::EXE_SUFFIX);
        let sibling = exe.with_file_name(&name);
        if sibling.exists() {
            return sibling;
        }
        let target = exe
            .parent()
            .and_then(Path::parent)
            .expect("target directory")
            .join("acceptance");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let built = Command::new(cargo)
            .args([
                "build",
                "--locked",
                "-p",
                "api",
                "--bin",
                "ensign-api",
                "--target-dir",
            ])
            .arg(&target)
            .status()
            .expect("cargo build ensign-api");
        assert!(built.success(), "cargo build ensign-api");
        target.join("debug").join(name)
    })
}

pub struct Belt {
    pub url: String,
    pub home: PathBuf,
}

impl Belt {
    pub fn drive(&self, args: &[&str], feed: Option<&str>) -> Run {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ensign"))
            .args(args)
            .env_clear()
            .env("HOME", &self.home)
            .env("ENSIGN_URL", &self.url)
            .env("ENSIGN_HOME", &self.home)
            .stdin(if feed.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("ensign process");
        if let Some(text) = feed {
            let mut pipe = child.stdin.take().expect("stdin");
            pipe.write_all(text.as_bytes()).expect("feed");
        }
        let done = child.wait_with_output().expect("ensign output");
        Run {
            code: done.status.code().unwrap_or(-1),
            out: String::from_utf8_lossy(&done.stdout).into_owned(),
            err: String::from_utf8_lossy(&done.stderr).into_owned(),
        }
    }
}
