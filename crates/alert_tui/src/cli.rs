use crate::{app, framework};
use argh::FromArgs;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::{
    io,
    panic::{set_hook, take_hook},
};

#[derive(FromArgs, Debug, Clone)]
#[argh(description = "Alert TUI")]
pub struct Args {
    #[argh(option, short = 'f', description = "target fps cap")]
    pub target_fps: Option<u16>,
}

fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

fn init_tui() -> io::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(io::stdout()))
}

pub fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

pub async fn run_tui(args: Args) -> anyhow::Result<()> {
    let config = framework::UiConfig::from_target_fps(args.target_fps);

    init_panic_hook();
    let mut terminal = init_tui()?;
    let (app, sender, receiver) = app::bootstrap(config).await;

    let result = framework::run_app(&mut terminal, app, config, receiver, sender).await;

    restore_tui()?;
    terminal
        .show_cursor()
        .map_err(|err| anyhow::anyhow!("failed to show cursor: {err}"))?;
    result
}
