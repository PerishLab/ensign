use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    let home = std::env::temp_dir().join(format!("ensign-usage-{}", std::process::id()));
    Command::new(env!("CARGO_BIN_EXE_ensign"))
        .args(args)
        .env("ENSIGN_HOME", home.join("data"))
        .env("HOME", home.join("home"))
        .output()
        .expect("run ensign")
}

fn bound() -> bool {
    option_env!("ENSIGN_BUILD_CHANNEL") != Some("unbound")
}

#[test]
fn unknown() {
    for verb in ["definitely-not-a-command", "--bogus"] {
        let output = run(&[verb]);
        assert!(!output.status.success(), "{verb}: {output:?}");
        if bound() {
            assert_eq!(output.status.code(), Some(2), "{verb}: {output:?}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains(&format!("unrecognized command '{verb}'")),
                "{error}"
            );
        }
        assert!(output.stdout.is_empty(), "{verb}: {output:?}");
    }
}

#[test]
fn help() {
    for args in [&[][..], &["help"], &["--help"], &["-h"]] {
        let output = run(args);
        if !bound() {
            assert!(!output.status.success(), "{args:?}: {output:?}");
            continue;
        }
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("login <login>"));
    }
}
