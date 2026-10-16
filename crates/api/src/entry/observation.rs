use plumb::observation::{Declaration, Run};
use std::path::PathBuf;

pub(crate) fn start(command: &str) -> Option<Run> {
    let home = plumb::config::value("ENSIGN_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("ensign"))?;
    if home.as_os_str().is_empty() {
        eprintln!("ensign-api observation: no home to read observation configuration");
        return None;
    }
    let declaration = Declaration {
        producer: "ensign",
        file: &home.join("config.toml"),
        buffer: home.join("state/locus"),
    };
    Run::start(declaration, command)
}
