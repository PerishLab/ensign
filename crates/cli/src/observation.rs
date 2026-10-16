use plumb::observation::{Declaration, Run};

pub(crate) fn start(command: &str) -> Option<Run> {
    let home = match crate::config::home() {
        Ok(home) => home,
        Err(error) => {
            eprintln!("ensign observation: {error}");
            return None;
        }
    };
    let declaration = Declaration {
        producer: "ensign",
        file: &home.join("config.toml"),
        buffer: home.join("state/locus"),
    };
    Run::start(declaration, command)
}
