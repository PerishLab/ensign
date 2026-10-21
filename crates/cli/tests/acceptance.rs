mod acceptance {
    mod cli;
    mod seat;
    #[path = "../../../api/tests/acceptance/rig/stage.rs"]
    mod stage;
    #[path = "../../../api/tests/acceptance/rig/wire.rs"]
    mod wire;
}
