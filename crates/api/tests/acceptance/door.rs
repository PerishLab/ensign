use super::bench::Bench;
use super::open;
use super::shaky::{DETAIL, Kind, Knob};
use super::wire::{Head, Reply, Wire, bare};
use serde_json::{Value, json};
use std::sync::Arc;

const CASES: [(Kind, u16, &str); 3] = [
    (Kind::Down, 503, "store unavailable"),
    (Kind::Taken, 409, "key taken"),
    (Kind::Fault, 500, "store fault"),
];
const SOURCES: &str = "FROM \"actor_source\"";
const PASSES: &str = "FROM \"source_pass\"";
const PROFILES: &str = "FROM \"actor_profile\"";
const PROFILE: &str = "INSERT INTO \"actor_profile\"";
const INVITE: &str = "INSERT INTO \"invite\"";

#[test]
fn namespace() {
    let (stage, wire) = open();
    let root = wire.fetch(&format!("{}/health", stage.origin), &bare());
    assert_eq!(root.status, 404, "root health");
}

#[test]
fn possession() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let seat = wire.post("/invite", json!({ "note": "first" }), &crown);
    let code = seat.text("code");
    assert_eq!(seat.status, 201, "invite");
    assert!(!code.is_empty(), "invite code");
    let open = wire.post("/invite", json!({ "note": "open" }), &bare());
    assert!(
        matches!(open.status, 401 | 403),
        "anon invite {}",
        open.status
    );
    let made = wire.post(
        "/join",
        json!({ "code": code, "login": "ada", "name": "Ada", "pass": "seaworthy" }),
        &bare(),
    );
    assert_eq!(made.status, 201, "join");
    let again = wire.post(
        "/join",
        json!({ "code": code, "login": "bob", "name": "Bob", "pass": "driftwood" }),
        &bare(),
    );
    assert_eq!(again.status, 404, "burned invite");
    let chit = wire.post("/invite", json!({ "note": "short" }), &crown);
    let feeble = wire.post(
        "/join",
        json!({ "code": chit.text("code"), "login": "wisp", "name": "Wisp", "pass": "short" }),
        &bare(),
    );
    assert_eq!(feeble.status, 400, "weak password admitted");
}

#[test]
fn floor() {
    let (stage, wire) = open();
    wire.enrol(&stage.crown(), "ada");
    let wrong = wire.post(
        "/login",
        json!({ "login": "ada", "pass": "adrift" }),
        &bare(),
    );
    assert_eq!(wrong.status, 401, "wrong pass");
    let right = wire.post(
        "/login",
        json!({ "login": "ada", "pass": "pass-ada" }),
        &bare(),
    );
    assert_eq!(right.status, 201, "login");
    let jar = vec![("cookie".to_string(), right.cookie())];
    let me = wire.get("/whoami", &jar);
    assert_eq!(me.status, 200, "whoami");
    assert_eq!(me.text("login"), "ada", "whoami login");
    let anon = wire.get("/whoami", &bare());
    assert_eq!(anon.status, 401, "anon whoami");
}

#[test]
fn logout() {
    let (stage, wire) = open();
    let face = wire.enrol(&stage.crown(), "ada");
    let out = wire.post("/logout", json!({}), &face.head);
    assert_eq!(out.status, 204, "logout");
    let gone = wire.get("/whoami", &face.head);
    assert_eq!(gone.status, 401, "dead session");
}

struct Rig {
    bench: Bench,
    wire: Wire,
    knob: Arc<Knob>,
}

impl Rig {
    fn new() -> Self {
        let knob = Arc::new(Knob::default());
        let bench = Bench::shaky(knob.clone());
        let wire = bench.wire();
        Self { bench, wire, knob }
    }

    fn code(&self, note: &str) -> String {
        let card = self
            .wire
            .post("/invite", json!({ "note": note }), &self.bench.crown());
        assert_eq!(card.status, 201, "invite");
        card.text("code")
    }

    fn join(&self, code: &str, login: &str) -> Reply {
        let body =
            json!({ "code": code, "login": login, "name": login, "pass": "seaworthy-anchor" });
        self.wire.post("/join", body, &bare())
    }

    fn enrol(&self, login: &str) {
        let made = self.join(&self.code(login), login);
        assert_eq!(made.status, 201, "join {login}");
    }

    fn login(&self, login: &str) -> Reply {
        let body = json!({ "login": login, "pass": "seaworthy-anchor" });
        self.wire.post("/login", body, &bare())
    }

    fn face(&self, login: &str) -> Head {
        self.enrol(login);
        let back = self.login(login);
        assert_eq!(back.status, 201, "login {login}");
        vec![("cookie".into(), back.cookie())]
    }
}

fn settle(reply: &Reply, status: u16, note: &str) {
    assert_eq!(reply.status, status, "status for {note}");
    assert_eq!(reply.body, json!({ "error": note }), "body for {note}");
    let text = reply.body.to_string();
    assert!(
        !text.contains(DETAIL) && !text.contains("db.internal"),
        "driver detail in {text}"
    );
}

#[test]
fn lookup() {
    let rig = Rig::new();
    for (at, (kind, status, note)) in CASES.iter().enumerate() {
        rig.knob.arm(SOURCES, *kind);
        let reply = rig.login(&format!("ghost{at}"));
        settle(&reply, *status, note);
    }
    rig.knob.clear();
    assert_eq!(
        rig.login("ghost").status,
        401,
        "unknown login after recovery"
    );
}

#[test]
fn guard() {
    let rig = Rig::new();
    for (at, (kind, status, note)) in CASES.iter().enumerate() {
        let login = format!("sailor{at}");
        rig.enrol(&login);
        rig.knob.arm(PASSES, *kind);
        settle(&rig.login(&login), *status, note);
        rig.knob.clear();
        assert_eq!(rig.login(&login).status, 201, "login after recovery");
    }
}

#[test]
fn invite() {
    let rig = Rig::new();
    for (kind, status, note) in CASES {
        rig.knob.arm(INVITE, kind);
        let reply = rig
            .wire
            .post("/invite", json!({ "note": "x" }), &rig.bench.crown());
        settle(&reply, status, note);
    }
    rig.knob.clear();
    assert!(!rig.code("again").is_empty(), "invite after recovery");
}

#[test]
fn enlist() {
    let rig = Rig::new();
    for (at, (kind, status, note)) in CASES.iter().enumerate() {
        let code = rig.code("enlist");
        rig.knob.arm(PROFILE, *kind);
        settle(&rig.join(&code, &format!("deck{at}")), *status, note);
        rig.knob.clear();
        assert_eq!(
            rig.join(&code, &format!("deck{at}")).status,
            201,
            "join after recovery"
        );
    }
}

#[test]
fn profile() {
    let rig = Rig::new();
    let head = rig.face("ada");
    for (kind, status, note) in CASES {
        rig.knob.arm(PROFILES, kind);
        settle(&rig.wire.get("/whoami", &head), status, note);
    }
    rig.knob.clear();
    assert_eq!(
        rig.wire.get("/whoami", &head).status,
        200,
        "whoami after recovery"
    );
}

#[test]
fn revive() {
    let rig = Rig::new();
    let head = rig.face("ada");
    let minted = rig.wire.post("/mint", json!({}), &head);
    let codes: Vec<Value> = minted.body["codes"].as_array().expect("codes").clone();
    for (at, (kind, status, note)) in CASES.iter().enumerate() {
        rig.knob.arm(PASSES, *kind);
        let body = json!({ "login": "ada", "code": codes[at], "pass": "refloor-anchor-8" });
        settle(&rig.wire.post("/revive", body, &bare()), *status, note);
    }
    rig.knob.clear();
    let body = json!({ "login": "ada", "code": codes[0], "pass": "refloor-anchor-8" });
    assert_eq!(
        rig.wire.post("/revive", body, &bare()).status,
        204,
        "revive after recovery"
    );
}

#[test]
fn plain() {
    let rig = Rig::new();
    rig.knob.arm(SOURCES, Kind::Plain);
    let reply = rig.login("ghost");
    assert_eq!(reply.status, 500, "plain failure status");
    assert_eq!(reply.body, json!(""), "plain failure body");
}

#[test]
fn clash() {
    let rig = Rig::new();
    let code = rig.code("clash");
    rig.enrol("keeper");
    let reply = rig.join(&code, "keeper");
    assert_eq!(reply.status, 409, "own conflict");
    assert_eq!(reply.body, json!(""), "own conflict body");
    assert_eq!(
        rig.join(&code, "keeper2").status,
        201,
        "invite spent by the clash"
    );
}
