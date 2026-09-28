use clap::{Parser, Subcommand};
use std::process::exit;

#[derive(Parser)]
#[command(name = "ensign-api", version = plumb::version!("ENSIGN"))]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(default_value = ".")]
    root: String,
    #[arg(long = "sidecar-stamp", hide = true, global = true)]
    stamp: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    Bootstrap {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long = "artifact", value_name = "NAME=DEST")]
        artifacts: Vec<String>,
    },
    Serve {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long = "artifact", value_name = "NAME=DEST")]
        artifacts: Vec<String>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(error) = plumb::identity!("ENSIGN") {
        eprintln!("ensign-api: {error}");
        exit(1);
    }
    let cli = Cli::parse();
    if let Err(error) = plumb::identity::ready() {
        eprintln!("ensign-api: {error}");
        exit(1);
    }
    let _stamp = cli.stamp;
    match cli.command {
        Some(Command::Bootstrap { root, artifacts }) => api::bootstrap(&root, &artifacts).await,
        Some(Command::Serve { root, artifacts }) => api::serve(&root, &artifacts).await,
        None => api::serve(&cli.root, &[]).await,
    }
}
