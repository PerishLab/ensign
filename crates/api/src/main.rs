use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(default_value = ".")]
    root: String,
    #[arg(long, hide = true)]
    sidecar_stamp: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let _stamp = cli.sidecar_stamp;
    api::sail(&cli.root).await;
}
