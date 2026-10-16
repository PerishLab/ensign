mod acceptance {
    mod cli;
    mod seat;
    #[path = "../../../api/tests/acceptance/stage.rs"]
    mod stage;
    #[path = "../../../api/tests/acceptance/wire.rs"]
    mod wire;
}
