// Prevents additional console window on Windows in release, DO NOT REMOVE!!
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app_lib::fiumi_lib;
use argh::{from_env, FromArgs};

#[derive(FromArgs)]
/// Controlla lo stato dei fiumi in emilia romagna
struct Args {
    /// esegui app in TUI
    #[argh(switch)]
    cli: bool,
}

fn main() {
    let args: Args = from_env();
    if args.cli {
        fiumi_lib::cli::run_cli().unwrap();
    } else {
        app_lib::run();
    }
}
