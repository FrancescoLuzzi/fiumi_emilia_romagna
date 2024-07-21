// Prevents additional console window on Windows in release, DO NOT REMOVE!!
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fiumi_lib;
mod launch_options;
use argh::{from_env, FromArgs};

#[derive(FromArgs)]
/// Launch also as cli
struct Args {
    /// whether or not to jump
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
