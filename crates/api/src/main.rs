use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(default_value = ".")]
    root: String,
    #[arg(long = "sidecar-stamp", hide = true)]
    stamp: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let _stamp = cli.stamp;
    api::sail(&cli.root).await;
}
