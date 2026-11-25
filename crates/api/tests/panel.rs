mod panel {
    #[path = "../acceptance/rig/bench.rs"]
    mod bench;
    #[allow(dead_code)]
    #[path = "../acceptance/oidc.rs"]
    mod oidc;
    #[allow(dead_code)]
    #[path = "../acceptance/rig/shaky.rs"]
    mod shaky;
    #[path = "../acceptance/rig/stage.rs"]
    mod stage;
    #[path = "../acceptance/rig/wire.rs"]
    mod wire;

    mod account;
    mod actor;
    mod guard;
    mod invite;
    mod manage;
    mod team;

    use bench::Bench;
    use wire::{Face, Head, Wire};

    fn open() -> (Bench, Wire) {
        keyed(&[])
    }

    fn keyed(slugs: &[&str]) -> (Bench, Wire) {
        let bench = Bench::with(slugs);
        let wire = bench.wire();
        (bench, wire)
    }

    fn admin(wire: &Wire, crown: &Head, login: &str) -> Face {
        let boss = wire.enrol(crown, login);
        let who = boss.id.to_string();
        for unit in ["Actor", "Invite", "Team", "App"] {
            wire.grant(crown, &who, ("*", unit));
        }
        boss
    }
}
