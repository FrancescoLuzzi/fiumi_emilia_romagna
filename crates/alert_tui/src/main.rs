use alert_tui::cli;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: cli::Args = argh::from_env();
    cli::run_tui(args).await.unwrap();
}
