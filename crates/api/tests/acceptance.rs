mod acceptance {
    #[path = "rig/bench.rs"]
    mod bench;
    mod door;
    mod guard;
    mod oidc;
    mod refresh;
    mod rescue;
    #[path = "rig/shaky.rs"]
    mod shaky;
    mod smoke;
    #[path = "rig/stage.rs"]
    mod stage;
    mod swap;
    mod team;
    mod token;
    #[path = "rig/wire.rs"]
    mod wire;

    fn open() -> (bench::Bench, wire::Wire) {
        keyed(&[])
    }

    fn keyed(slugs: &[&str]) -> (bench::Bench, wire::Wire) {
        let bench = bench::Bench::with(slugs);
        let wire = bench.wire();
        (bench, wire)
    }

    fn staged(slugs: &[&str]) -> (stage::Stage, wire::Wire) {
        let exe = std::path::Path::new(env!("CARGO_BIN_EXE_ensign-api"));
        let stage = stage::Stage::with(exe, slugs);
        let wire = wire::Wire::new(&stage.base);
        (stage, wire)
    }
}
