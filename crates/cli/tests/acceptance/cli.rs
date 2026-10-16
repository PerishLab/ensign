use super::seat::{Belt, api};
use super::stage::Stage;
use super::wire::Wire;
use serde_json::{Value, json};

fn open(home: &str) -> (Stage, Wire, Belt) {
    let stage = Stage::new(api());
    let wire = Wire::new(&stage.base);
    let belt = Belt {
        url: stage.base.clone(),
        home: stage.nest.join(home),
    };
    (stage, wire, belt)
}

fn card(wire: &Wire, stage: &Stage, note: &str) -> String {
    let seat = wire.post("/invite", json!({ "note": note }), &stage.crown());
    let code = seat.text("code");
    assert_eq!(seat.status, 201, "invite");
    assert!(!code.is_empty(), "invite code");
    code
}

#[test]
fn trip() {
    let (stage, wire, belt) = open("cli");
    let crown = stage.crown();
    let pilot = wire.enrol(&crown, "pilot");
    for unit in ["Invite", "Team", "App"] {
        wire.grant(&crown, &pilot.id.to_string(), ("put", unit));
    }
    let login = belt.drive(&["login", "pilot"], Some("pass-pilot"));
    assert_eq!(login.code, 0, "cli login: {}", login.err.trim());
    let who = belt.drive(&["whoami"], None);
    assert!(
        who.code == 0 && who.out.contains("pilot"),
        "cli whoami {}: {}",
        who.code,
        who.out.trim()
    );
    let inv = belt.drive(&["invite", "cli-made"], None);
    assert!(
        inv.code == 0 && inv.out.trim().len() >= 32,
        "cli invite {}: {}",
        inv.code,
        inv.out.trim()
    );
    let team = belt.drive(&["team", "flight"], None);
    assert_eq!(team.code, 0, "cli team {}", team.err.trim());
    let app = belt.drive(
        &[
            "app",
            "Deck",
            "deck",
            "https://deck.lab",
            "https://deck.lab/cb",
        ],
        None,
    );
    assert_eq!(app.code, 0, "cli app {}", app.err.trim());
    let apps = belt.drive(&["apps"], None);
    assert!(
        apps.out.contains("deck"),
        "cli apps missing deck: {}",
        apps.out.trim()
    );
    let out = belt.drive(&["logout"], None);
    assert_eq!(out.code, 0, "cli logout {}", out.err.trim());
    let gone = belt.drive(&["whoami"], None);
    assert_ne!(gone.code, 0, "cli whoami survived logout");
}

#[test]
fn redeem() {
    let (stage, wire, belt) = open("cli-join");
    let code = card(&wire, &stage, "cli-join");
    let made = belt.drive(&["join", &code, "cadet", "Cadet"], Some("pass-cadet"));
    assert!(
        made.code == 0 && made.out.contains("cadet"),
        "cli join {}: {}",
        made.code,
        made.err.trim()
    );
    let first = belt.drive(&["login", "cadet"], Some("pass-cadet"));
    assert_eq!(first.code, 0, "cli login after join {}", first.err.trim());
    let kept = std::fs::read_to_string(belt.home.join("state/pat.json")).expect("pat.json");
    let held: Value = serde_json::from_str(&kept).expect("credential json");
    assert_eq!(held["schema"], json!(1), "credential schema");
    let pat = held["token"].as_str().unwrap_or_default();
    let minted = wire.post(
        "/mint",
        json!({}),
        &vec![("authorization".into(), format!("token {pat}"))],
    );
    let spare = minted.body["codes"][0]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(
        minted.status == 201 && !spare.is_empty(),
        "mint {}",
        minted.status
    );
    let back = belt.drive(&["recover", "cadet", &spare], Some("pass-cadet-two"));
    assert_eq!(back.code, 0, "cli recover: {}", back.err.trim());
    let stale = belt.drive(&["login", "cadet"], Some("pass-cadet"));
    assert_ne!(
        stale.code, 0,
        "the old password still signs in after recovery"
    );
    let fresh = belt.drive(&["login", "cadet"], Some("pass-cadet-two"));
    assert_eq!(
        fresh.code,
        0,
        "cli login after recovery {}",
        fresh.err.trim()
    );
    let twice = belt.drive(&["recover", "cadet", &spare], Some("pass-cadet-three"));
    assert_ne!(
        twice.code, 0,
        "a burned rescue code recovered a second time"
    );
}

#[test]
fn crown() {
    let (stage, wire, belt) = open("cli-crown");
    let code = card(&wire, &stage, "cli-crown");
    let made = belt.drive(&["join", &code, "regent", "Regent"], Some("pass-regent"));
    assert_eq!(made.code, 0, "cli join {}", made.err.trim());
    let first = belt.drive(&["login", "regent"], Some("pass-regent"));
    assert_eq!(first.code, 0, "cli login {}", first.err.trim());
    let before = belt.drive(&["team", "regency"], None);
    assert_ne!(
        before.code, 0,
        "an ordinary actor founded a team before being crowned"
    );
    let wrong = belt.drive(&["crown", "regent"], Some("deadbeef"));
    assert_ne!(wrong.code, 0, "a bad sudo token crowned an actor");
    let rite = belt.drive(&["crown", "regent"], Some(&stage.sudo));
    assert_eq!(rite.code, 0, "cli crown: {}", rite.err.trim());
    let after = belt.drive(&["team", "regency"], None);
    assert_eq!(
        after.code,
        0,
        "crowned actor still refused {}",
        after.err.trim()
    );
}

#[test]
fn stranger() {
    let (stage, wire, belt) = open("cli");
    wire.enrol(&stage.crown(), "stow");
    let login = belt.drive(&["login", "stow"], Some("pass-stow"));
    assert_eq!(login.code, 0, "cli login {}", login.err.trim());
    let app = belt.drive(
        &["app", "Sneak", "sneak", "https://x.lab", "https://x.lab/cb"],
        None,
    );
    assert!(
        app.code != 0 && app.err.contains("refused"),
        "stranger app {}: {}",
        app.code,
        app.err.trim()
    );
    belt.drive(&["logout"], None);
}
