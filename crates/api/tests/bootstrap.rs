use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Nest(PathBuf);

impl Nest {
    fn new() -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("ensign-bootstrap-{}-{serial}", std::process::id()));
        std::fs::create_dir(&path).expect("temp nest");
        std::fs::write(
            path.join("keel.toml"),
            "[store]\nkind = \"file\"\npath = \"estate.db\"\n\n\
             [identity]\nunit = \"Actor\"\n",
        )
        .expect("config");
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Nest {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn bootstrap(root: &Path, sudo: &Path, signing: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_api"))
        .arg("bootstrap")
        .arg(root)
        .arg("--artifact")
        .arg(format!("sudo=file:{}", sudo.display()))
        .arg("--artifact")
        .arg(format!("signing=file:{}", signing.display()))
        .output()
        .expect("bootstrap process")
}

#[test]
fn replay() {
    let nest = Nest::new();
    let sudo = nest.path("sudo");
    let signing = nest.path("sign.pem");

    let first = bootstrap(&nest.0, &sudo, &signing);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let held = std::fs::read(&sudo).expect("sudo");
    let pem = std::fs::read(&signing).expect("signing");
    assert!(!first.stderr.windows(held.len()).any(|part| part == held));

    let second = bootstrap(&nest.0, &sudo, &signing);
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(std::fs::read(&sudo).expect("sudo replay"), held);
    assert_eq!(std::fs::read(&signing).expect("signing replay"), pem);
}

#[test]
fn occupied() {
    let nest = Nest::new();
    let sudo = nest.path("sudo");
    let signing = nest.path("sign.pem");
    assert!(bootstrap(&nest.0, &sudo, &signing).status.success());

    let lost = nest.path("replacement-sudo");
    let replay = bootstrap(&nest.0, &lost, &signing);
    assert!(!replay.status.success());
    assert!(!lost.exists());
    assert!(
        String::from_utf8_lossy(&replay.stderr)
            .contains("sudo custody is absent for an occupied estate")
    );
}

#[test]
fn malformed() {
    let nest = Nest::new();
    let sudo = nest.path("sudo");
    let signing = nest.path("sign.pem");
    let malformed = b"not a private key";
    std::fs::write(&signing, malformed).expect("malformed signing");

    let first = bootstrap(&nest.0, &sudo, &signing);
    assert!(!first.status.success());
    assert_eq!(std::fs::read(&signing).expect("preserved"), malformed);

    let replay = bootstrap(&nest.0, &sudo, &signing);
    assert!(!replay.status.success());
    assert_eq!(
        std::fs::read(&signing).expect("replay preserved"),
        malformed
    );
}

fn spawn(root: &Path, sudo: &Path, signing: &Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_api"))
        .arg("bootstrap")
        .arg(root)
        .arg("--artifact")
        .arg(format!("sudo=file:{}", sudo.display()))
        .arg("--artifact")
        .arg(format!("signing=file:{}", signing.display()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bootstrap process")
}

#[test]
fn severed() {
    for pause in [10, 30, 60, 90, 120, 150, 200] {
        let nest = Nest::new();
        let sudo = nest.path("sudo");
        let signing = nest.path("sign.pem");
        let mut early = spawn(&nest.0, &sudo, &signing);
        std::thread::sleep(std::time::Duration::from_millis(pause));
        let _ = early.kill();
        let _ = early.wait();
        let again = bootstrap(&nest.0, &sudo, &signing);
        assert!(
            again.status.success(),
            "a bootstrap severed at {pause}ms left an estate that cannot be replayed: {}",
            String::from_utf8_lossy(&again.stderr)
        );
        let third = bootstrap(&nest.0, &sudo, &signing);
        assert!(
            third.status.success(),
            "replay after a {pause}ms severance is not idempotent: {}",
            String::from_utf8_lossy(&third.stderr)
        );
        let kept = std::fs::read_to_string(&sudo).expect("sudo custody");
        assert_eq!(
            kept.trim().len(),
            64,
            "sudo custody is not one 64-hex token after a {pause}ms severance"
        );
    }
}

#[test]
fn contended() {
    let nest = Nest::new();
    let sudo = nest.path("sudo");
    let signing = nest.path("sign.pem");
    let held: Vec<Child> = (0..3).map(|_| spawn(&nest.0, &sudo, &signing)).collect();
    let reaped: Vec<Output> = held
        .into_iter()
        .map(|child| child.wait_with_output().expect("wait"))
        .collect();
    let won = reaped.iter().filter(|out| out.status.success()).count();
    assert!(
        won >= 1,
        "no contender sealed the estate: {:?}",
        reaped
            .iter()
            .map(|out| String::from_utf8_lossy(&out.stderr).to_string())
            .collect::<Vec<_>>()
    );
    let after = bootstrap(&nest.0, &sudo, &signing);
    assert!(
        after.status.success(),
        "contention left an estate that cannot be replayed: {}",
        String::from_utf8_lossy(&after.stderr)
    );
    let kept = std::fs::read_to_string(&sudo).expect("sudo custody");
    assert_eq!(
        kept.trim().len(),
        64,
        "sudo custody is not one 64-hex token"
    );
}
