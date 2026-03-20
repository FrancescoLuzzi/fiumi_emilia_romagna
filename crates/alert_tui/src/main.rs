use alert_tui::{cli, config};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: config::Args = argh::from_env();
    cli::run_tui(args).await.unwrap();
}
