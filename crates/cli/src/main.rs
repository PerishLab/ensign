mod config;
mod deed;
mod wire;

use clap::Parser;
use deed::{Rest, list, logout, whoami};
use std::process::exit;
use wire::Reply;

#[derive(Parser)]
#[command(
    disable_help_flag = true,
    disable_version_flag = true,
    trailing_var_arg = true
)]
struct Cli {
    #[arg(allow_hyphen_values = true)]
    args: Vec<String>,
}

fn main() {
    if let Err(error) = plumb::identity!("ENSIGN") {
        eprintln!("ensign: {error}");
        exit(1);
    }
    let args = Cli::parse().args;
    let verb = args.first().map(String::as_str).unwrap_or("help");
    if !matches!(verb, "--version" | "-V")
        && let Err(error) = plumb::identity::ready()
    {
        eprintln!("ensign: {error}");
        exit(1);
    }
    let rest = Rest(&args[args.len().min(1)..]);
    let done = match verb {
        "--version" | "-V" => stamp(),
        "login" => rest.login(),
        "logout" => logout(),
        "whoami" => whoami(),
        "invite" => rest.invite(),
        "crown" => rest.crown(),
        "join" => rest.join(),
        "recover" => rest.recover(),
        "actors" => list("Actor", &["login", "name", "kind"]),
        "teams" => list("Team", &["name"]),
        "team" => rest.team(),
        "apps" => list("App", &["name", "slug", "mode"]),
        "app" => rest.app(),
        _ => usage(),
    };
    if let Err(note) = done {
        eprintln!("ensign: {note}");
        exit(1);
    }
}

fn stamp() -> Reply {
    println!("ensign {}", plumb::version!("ENSIGN"));
    Ok(())
}

fn usage() -> Reply {
    eprintln!("ensign — a client for the ensign identity provider");
    eprintln!();
    eprintln!("  login <login>        sign in; reads the password from stdin");
    eprintln!("  logout               revoke this token and forget it");
    eprintln!("  whoami               show the signed-in identity");
    eprintln!("  invite [note]        open an invite code");
    eprintln!("  crown <login>        grant site admin; reads the sudo token from stdin");
    eprintln!("  join <code> <login> <name>   redeem an invite; reads the password from stdin");
    eprintln!(
        "  recover <login> <code>       burn a rescue code; reads the new password from stdin"
    );
    eprintln!("  actors               list actors");
    eprintln!("  teams                list teams");
    eprintln!("  team <name>          found a team");
    eprintln!("  apps                 list apps");
    eprintln!("  app <name> <slug> <home> <redirect>   register an oidc app");
    Ok(())
}
