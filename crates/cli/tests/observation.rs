#[path = "observation/fixture.rs"]
mod fixture;
#[path = "observation/server.rs"]
mod server;

use fixture::Fixture;

const ENABLED: &str = "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n";

#[test]
fn disabled() {
    let fixture = Fixture::new();
    let absent = fixture.run(&["whoami"]);
    fixture.configure("[locus]\nenabled = false\n");
    let disabled = fixture.run(&["whoami"]);
    assert_eq!(absent.status.code(), Some(0));
    assert_eq!(absent.stdout, disabled.stdout);
    assert_eq!(absent.stderr, disabled.stderr);
    assert_eq!(absent.status.code(), disabled.status.code());
    assert!(!fixture.home().join("state/locus").exists());
}

#[test]
fn buffered() {
    let fixture = Fixture::new();
    fixture.configure(ENABLED);
    assert_eq!(fixture.run(&["whoami"]).status.code(), Some(0));
    assert_eq!(
        fixture.run(&["login", "private-login"]).status.code(),
        Some(0)
    );
    let path = fixture.home().join("state/pat.json");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["token"], "private-token");
    assert_eq!(value["base"], fixture.server.url);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    fixture.server.status(401);
    assert_eq!(fixture.run(&["actors"]).status.code(), Some(1));
    assert_eq!(
        fixture.run(&["cookbook", "private-code"]).status.code(),
        Some(1)
    );
    assert_eq!(fixture.run(&["skill", "list"]).status.code(), Some(0));
    assert_eq!(fixture.run(&["login"]).status.code(), Some(1));
    fixture.credential("http://127.0.0.1:0/api");
    assert_eq!(fixture.run(&["whoami"]).status.code(), Some(1));
    let atoms = fixture.records();
    assert!(atoms.iter().all(|atom| atom["producer"] == "ensign"));
    for edge in ["enter", "return"] {
        assert_eq!(
            atoms
                .iter()
                .filter(|atom| atom["source"]["edge"] == edge)
                .count(),
            15
        );
    }
    for name in ["whoami", "login", "actors", "cookbook", "skill"] {
        assert!(
            atoms
                .iter()
                .any(|atom| atom["context"]["ensign.command"] == name)
        );
    }
    for code in [0, 1] {
        assert!(atoms.iter().any(
            |atom| atom["payload"]["event"] == "cli.finish" && atom["payload"]["code"] == code
        ));
    }
    let text = serde_json::to_string(&atoms).unwrap();
    for private in [
        "private-token",
        "private-password",
        "private-login",
        "private-name",
        "private-code",
        "127.0.0.1",
        "pat.json",
    ] {
        assert!(!text.contains(private));
    }
}

#[test]
fn isolated() {
    let fixture = Fixture::new();
    let baseline = fixture.run(&["whoami"]);
    let credential = std::fs::read(fixture.home().join("state/pat.json")).unwrap();
    for text in [
        ENABLED,
        "[locus]\nenabled = 'private-setting'\n",
        "[locus]\nenabled = true\nendpoint = ''\n",
    ] {
        fixture.configure(text);
        let result = fixture.run(&["whoami"]);
        assert_eq!(result.status.code(), baseline.status.code());
        assert_eq!(result.stdout, baseline.stdout);
        assert!(!result.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("private-setting"));
        assert_eq!(
            std::fs::read(fixture.home().join("state/pat.json")).unwrap(),
            credential
        );
        fixture.server.status(403);
        assert_eq!(fixture.run(&["actors"]).status.code(), Some(1));
        fixture.server.status(200);
    }
    let blocked = Fixture::new();
    blocked.configure(ENABLED);
    std::fs::write(blocked.home().join("state/locus"), "occupied").unwrap();
    let result = blocked.run(&["whoami"]);
    assert_eq!(result.status.code(), Some(0));
    assert!(!result.stderr.is_empty());
}
