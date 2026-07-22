use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(default_value = ".")]
    root: String,
}

#[tokio::main]
async fn main() {
    let _root = Cli::parse().root;
    api::sail().await;
}
