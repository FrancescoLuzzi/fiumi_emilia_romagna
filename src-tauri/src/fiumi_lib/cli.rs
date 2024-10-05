//! # [Ratatui] Table example
//!
//! The latest version of this example is available in the [examples] folder in the repository.
//!
//! Please note that the examples are designed to be run against the `main` branch of the Github
//! repository. This means that you may not be able to compile with the latest release version on
//! crates.io, or the one that you have installed locally.
//!
//! See the [examples readme] for more information on finding examples that match the version of the
//! library you are using.
//!
//! [Ratatui]: https://github.com/ratatui-org/ratatui
//! [examples]: https://github.com/ratatui-org/ratatui/blob/main/examples
//! [examples readme]: https://github.com/ratatui-org/ratatui/blob/main/examples/README.md

use crate::{
    api::{get_station_timeseries, get_stations},
    fiumi_lib::{
        event_handler_trait::MutStatefulEventHandler,
        graph::{GraphPage, GraphPageState},
        table::{SelectionPage, SelectionPageState},
    },
    model::Stations,
};
use chrono::{DurationRound, TimeDelta};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::Rect,
    terminal::{Frame, Terminal},
    widgets::StatefulWidgetRef,
};
use std::{
    error::Error,
    io,
    ops::ControlFlow,
    panic::{set_hook, take_hook},
    time::Duration,
};

enum Page {
    Selection(SelectionPageState),
    Graph(GraphPageState),
}

impl Page {
    fn render(&mut self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        match self {
            Page::Selection(state) => SelectionPage {}.render_ref(area, buf, state),
            Page::Graph(state) => GraphPage {}.render_ref(area, buf, state),
        }
    }
}

struct App<const N: usize> {
    pages: [Option<Page>; N],
    page_idx: usize,
}

impl<const N: usize> App<N> {
    pub fn new(pages: [Option<Page>; N]) -> Self {
        Self { pages, page_idx: 0 }
    }
    pub fn render(&mut self, frame: &mut Frame) {
        self.pages[self.page_idx]
            .as_mut()
            .unwrap()
            .render(frame.size(), frame.buffer_mut());
    }
    pub fn handle_event(&mut self, event: Event) -> ControlFlow<(), ()> {
        match &mut self.pages[self.page_idx].as_mut().unwrap() {
            Page::Selection(state) => match (SelectionPage {}).handle_mut(event, Some(state)) {
                ControlFlow::Continue(data) => {
                    if data.is_some() {
                        // TODO: create GraphPageState getting the data from the api
                        let station = state.get_selected_data().unwrap();
                        let data = get_station_timeseries(&station).unwrap();
                        self.pages[1] = Some(Page::Graph(GraphPageState::new(station, data)));
                        self.page_idx = 1;
                    }
                    ControlFlow::Continue(())
                }
                ControlFlow::Break(_) => ControlFlow::Break(()),
            },
            Page::Graph(state) => {
                if let ControlFlow::Break(_) = (GraphPage {}).handle_mut(event, Some(state)) {
                    self.page_idx = 0;
                }
                ControlFlow::Continue(())
            }
        }
    }
}

pub fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        // intentionally ignore errors here since we're already in a panic
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}
pub fn init_tui() -> io::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(io::stdout()))
}

pub fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn run_app<B: Backend, const N: usize>(
    terminal: &mut Terminal<B>,
    mut app: App<N>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| app.render(f))?;
        if crossterm::event::poll(Duration::new(0, 1_500_000))? {
            if let ControlFlow::Break(_) = app.handle_event(event::read()?) {
                return Ok(());
            }
        }
    }
}

pub fn run_cli() -> Result<(), Box<dyn Error>> {
    let mut now = chrono::Local::now();
    let delta_15_mins = TimeDelta::try_minutes(15).unwrap();
    now -= delta_15_mins;
    now = now.duration_trunc(TimeDelta::try_minutes(15).unwrap())?;
    let mut stations: Stations = get_stations(now)?;
    while stations
        .0
        .iter()
        .filter(|s| s.value().is_some())
        .count()
        .lt(&10)
    {
        now -= delta_15_mins;
        stations = get_stations(now)?;
    }
    if stations.0.is_empty() {
        return Ok(());
    }
    stations.0.sort_by(|a, b| b.cmp(a));
    init_panic_hook();
    // setup terminal
    let mut terminal = init_tui()?;

    // create app and run it
    let app = App::<2>::new([
        Some(Page::Selection(SelectionPageState::new(stations.0))),
        None,
    ]);
    let res = run_app(&mut terminal, app);

    // restore terminal
    restore_tui()?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
