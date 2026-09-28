pub use plumb::skill::command::Deed;
use plumb::skill::{Kit, command::Command};
use std::path::PathBuf;

const RELEASES: &str = "https://releases.ensign.perish.uk";
const DEPOT: &str = "https://depot.ensign.perish.uk";

struct Rig {
    home: String,
    releases: String,
}

impl Rig {
    fn resolve() -> Self {
        Self {
            home: plumb::config::value("ENSIGN_HOME").unwrap_or_else(|| {
                plumb::config::data("ensign")
                    .map(|path| path.display().to_string())
                    .unwrap_or_default()
            }),
            releases: plumb::config::value("ENSIGN_RELEASES")
                .unwrap_or_else(|| RELEASES.to_string()),
        }
    }
}

pub fn run(deed: Deed) -> i32 {
    let rig = Rig::resolve();
    if rig.home.is_empty() {
        eprintln!("ensign skill: no data home; set ENSIGN_HOME");
        return 1;
    }
    let kit = Kit {
        name: "ensign".to_string(),
        home: plumb::config::home().unwrap_or_else(|| PathBuf::from(".")),
        state: PathBuf::from(&rig.home).join("state").join("skills.json"),
        url: rig.releases,
    };
    let depot = kit.depot(DEPOT, "ensign", plumb::version!("ENSIGN"));
    Command("ensign").run(&depot, deed)
}
