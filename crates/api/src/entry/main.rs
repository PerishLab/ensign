mod observation;
use clap::{Parser, Subcommand};
use std::process::exit;

#[derive(Parser)]
#[command(name = "ensign-api", version = plumb::version!("ENSIGN"), after_help = "Only finite bootstrap CLI observation is supported, off by default. Set [locus] enabled = true in config.toml under ENSIGN_HOME or the platform .ensign home, separate from ensign.toml and artifact custody. Buffer is state/locus; endpoint overrides LOCUS_API, then http://127.0.0.1:43308. Serving does not initialize an observer. Refusals preserve process exit and may leave an unclosed Source.")]
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
        Some(Command::Bootstrap { root, artifacts }) => {
            let observer = observation::Run::start("api.bootstrap");
            bootstrap(&root, &artifacts, observer.as_ref()).await;
            if let Some(observer) = observer {
                observer.finish(0);
            }
        }
        Some(Command::Serve { root, artifacts }) => api::serve(&root, &artifacts).await,
        None => api::serve(&cli.root, &[]).await,
    }
}

#[locus::trace(with = observer.map(observation::Run::view))]
async fn bootstrap(root: &str, artifacts: &[String], observer: Option<&observation::Run>) {
    api::bootstrap(root, artifacts).await;
}
