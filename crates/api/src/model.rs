use keel::atom::string;
use keel::atom::url as link;
use keel::resource;

#[resource]
pub(crate) struct Actor {
    #[field(string, unique)]
    sub: string,
    #[field(string)]
    kind: string,
    #[field(bool)]
    barred: bool,
}

#[resource(veil)]
pub(crate) struct Profile {
    #[field(string, unique)]
    handle: string,
    #[field(string)]
    name: string,
    #[relation(Actor, one2one, root)]
    actor: Actor,
}

#[resource(veil)]
pub(crate) struct Source {
    #[field(string)]
    kind: string,
    #[field(string, unique = kind)]
    handle: string,
    #[relation(Actor, many2one, root)]
    actor: Actor,
}

#[resource(veil)]
pub(crate) struct Pass {
    #[field(string)]
    hash: string,
    #[relation(Source, one2one, root)]
    source: Source,
}

#[resource(veil)]
pub(crate) struct Invite {
    #[field(string, unique)]
    hash: string,
    #[field(string)]
    note: string,
}

#[resource(veil)]
pub(crate) struct Rescue {
    #[field(string)]
    hash: string,
    #[relation(Actor, many2one, root)]
    actor: Actor,
}

#[resource]
pub(crate) struct Team {
    #[field(string, unique)]
    name: string,
    #[relation(Actor, many2many, crew)]
    members: Actor,
}

#[resource]
pub(crate) struct App {
    #[field(string)]
    name: string,
    #[field(string, unique)]
    slug: string,
    #[field(url)]
    home: link,
    #[field(url)]
    redirect: link,
    #[field(string)]
    secret: string,
    #[field(string)]
    mode: string,
}

#[resource(veil)]
pub(crate) struct Renew {
    #[field(string, unique)]
    hash: string,
    #[field(string)]
    slug: string,
    #[field(string)]
    scope: string,
    #[relation(Actor, many2one, root)]
    actor: Actor,
}

keel_gate::gate!(Actor);
