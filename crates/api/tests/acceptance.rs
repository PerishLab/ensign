mod acceptance {
    mod door;
    mod guard;
    mod oidc;
    mod refresh;
    mod rescue;
    mod stage;
    mod team;
    mod token;
    mod wire;

    fn open() -> (stage::Stage, wire::Wire) {
        let exe = std::path::Path::new(env!("CARGO_BIN_EXE_ensign-api"));
        let stage = stage::Stage::new(exe);
        let wire = wire::Wire::new(&stage.base);
        (stage, wire)
    }
}
