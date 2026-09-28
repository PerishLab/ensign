mod config;
mod cookbook;
mod deed;
mod skill;
mod wire;

use clap::{Parser, error::ErrorKind};
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

#[derive(Parser)]
#[command(name = "ensign skill", disable_version_flag = true)]
struct Skill {
    #[command(subcommand)]
    deed: skill::Deed,
}

#[derive(Parser)]
#[command(name = "ensign cookbook", disable_version_flag = true)]
struct Cookbook {
    code: Option<String>,
    #[arg(long)]
    json: bool,
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
        "cookbook" => recovery(&args[args.len().min(1)..]),
        "skill" => managed(&args[args.len().min(1)..]),
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

fn recovery(args: &[String]) -> Reply {
    let cli = match Cookbook::try_parse_from(
        std::iter::once("ensign cookbook".to_string()).chain(args.iter().cloned()),
    ) {
        Ok(cli) => cli,
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            error.print().map_err(|held| held.to_string())?;
            return Ok(());
        }
        Err(error) => return Err(error.to_string()),
    };
    let text = cookbook::render(cli.code.as_deref(), cli.json)?;
    println!("{text}");
    Ok(())
}

fn managed(args: &[String]) -> Reply {
    let cli = match Skill::try_parse_from(
        std::iter::once("ensign skill".to_string()).chain(args.iter().cloned()),
    ) {
        Ok(cli) => cli,
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            error.print().map_err(|held| held.to_string())?;
            return Ok(());
        }
        Err(error) => return Err(error.to_string()),
    };
    let code = skill::run(cli.deed);
    if code != 0 {
        exit(code);
    }
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
    eprintln!("  cookbook [code]      read exact complex recovery");
    eprintln!("  skill <operation>    manage installed Ensign Skills");
    Ok(())
}
