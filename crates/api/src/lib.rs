mod artifact;
mod booth;
mod config;
mod door;
mod model;
mod oidc;
mod startup;
mod util;

pub use booth::Booth;
pub use startup::{Berth, bootstrap, serve};
pub use util::lock;

use crate::model::{Actor, App, Invite, Pass, Profile, Renew, Rescue, Source, Team, plug};
use keel::Graph;

pub fn shape() -> Graph {
    let mut graph = Graph::new();
    graph
        .plug::<Actor>()
        .plug::<Profile>()
        .plug::<Source>()
        .plug::<Pass>()
        .plug::<Invite>()
        .plug::<Rescue>()
        .plug::<Team>()
        .plug::<App>()
        .plug::<Renew>();
    plug(&mut graph);
    graph
}
