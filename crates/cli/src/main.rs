mod config;
mod cookbook;
mod deed;
mod observation;
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
    let observer = observation::Run::start(name(verb));
    let code = execute(&args);
    if let Some(observer) = observer {
        observer.finish(code);
    }
    if code != 0 {
        exit(code);
    }
}

fn name(verb: &str) -> &'static str {
    match verb {
        "--version" | "-V" => "version",
        "login" => "login",
        "logout" => "logout",
        "whoami" => "whoami",
        "invite" => "invite",
        "crown" => "crown",
        "join" => "join",
        "recover" => "recover",
        "actors" => "actors",
        "teams" => "teams",
        "team" => "team",
        "apps" => "apps",
        "app" => "app",
        "cookbook" => "cookbook",
        "skill" => "skill",
        _ => "help",
    }
}

#[locus::trace(with = observation::view())]
fn execute(args: &[String]) -> i32 {
    let verb = args.first().map(String::as_str).unwrap_or("help");
    if verb == "skill" {
        return match managed(&args[args.len().min(1)..]) {
            Ok(code) => code,
            Err(note) => {
                eprintln!("ensign: {note}");
                1
            }
        };
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
        "help" | "--help" | "-h" => {
            usage();
            Ok(())
        }
        _ => {
            eprintln!("ensign: unrecognized command '{verb}'");
            usage();
            return 2;
        }
    };
    match done {
        Ok(()) => 0,
        Err(note) => {
            eprintln!("ensign: {note}");
            1
        }
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

fn managed(args: &[String]) -> Result<i32, String> {
    let cli = match Skill::try_parse_from(
        std::iter::once("ensign skill".to_string()).chain(args.iter().cloned()),
    ) {
        Ok(cli) => cli,
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            error.print().map_err(|held| held.to_string())?;
            return Ok(0);
        }
        Err(error) => return Err(error.to_string()),
    };
    Ok(skill::run(cli.deed))
}

fn usage() {
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
    eprintln!();
    eprintln!(
        "Observation is off by default. Set [locus] enabled = true in config.toml under ENSIGN_HOME or the platform ensign data home (~/.ensign on Unix, LOCALAPPDATA/ensign on Windows)."
    );
    eprintln!(
        "endpoint overrides LOCUS_API, then http://127.0.0.1:43308; buffer is state/locus. Configuration is separate from state/pat.json and observation failures preserve client state and exit status."
    );
}
