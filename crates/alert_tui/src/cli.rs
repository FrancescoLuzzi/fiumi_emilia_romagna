use crate::{app, config};
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

pub async fn run_tui(args: config::Args) -> anyhow::Result<()> {
    let config = config::UiConfig::from_args(&args);

    init_panic_hook();
    let mut terminal = init_tui()?;
    let (app, sender, receiver) = app::bootstrap(config).await;

    let result = app::run_app(&mut terminal, app, config, receiver, sender).await;

    restore_tui()?;
    terminal
        .show_cursor()
        .map_err(|err| anyhow::anyhow!("failed to show cursor: {err}"))?;
    result
}
