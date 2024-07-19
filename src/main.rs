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

use std::{
    default::Default,
    error::Error,
    io,
    ops::ControlFlow,
    panic::{set_hook, take_hook},
    time::Duration,
};

use allerta_meteo::{
    event_handler_trait::MutStatefulEventHandler,
    graph::{GraphPage, GraphPageState},
    table::{SelectionPage, SelectionPageState},
    Station, TimeSeries, TimeValue,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::Rect,
    style::{palette::tailwind, Color},
    terminal::{Frame, Terminal},
    widgets::StatefulWidgetRef,
};

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];
#[derive(Default)]
pub struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    pub const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_style_fg: color.c400,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}
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
    colors: TableColors,
    color_index: usize,
}

impl<const N: usize> App<N> {
    pub fn new(pages: [Option<Page>; N]) -> Self {
        Self {
            pages,
            page_idx: 0,
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
        }
    }
    pub fn render(&mut self, frame: &mut Frame) {
        self.pages[self.page_idx]
            .as_mut()
            .unwrap()
            .render(frame.size(), frame.buffer_mut());
    }
    pub fn handle_event(&mut self, event: Event) -> ControlFlow<(), ()> {
        match &mut self.pages[self.page_idx].as_mut().unwrap() {
            Page::Selection(state) => match (SelectionPage {}).handle(event, Some(state)) {
                ControlFlow::Continue(data) => {
                    if data.is_some() {
                        // TODO: create GraphPageState getting the data from the api
                        let station = state.get_selected_data().unwrap();
                        let values:Vec<_> = reqwest::blocking::get(format!("https://allertameteo.regione.emilia-romagna.it/o/api/allerta/get-time-series/?stazione={}&variabile=254,0,0/1,-,-,-/B13215",station.idstazione())).unwrap().json::<Vec<TimeValue>>().unwrap();
                        self.pages[1] = Some(Page::Graph(GraphPageState::new(
                            station,
                            TimeSeries::new(values),
                        )));
                        self.page_idx = 1;
                    }
                    ControlFlow::Continue(())
                }
                ControlFlow::Break(_) => ControlFlow::Break(()),
            },
            Page::Graph(state) => {
                if let ControlFlow::Break(_) = (GraphPage {}).handle(event, Some(state)) {
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

fn main() -> Result<(), Box<dyn Error>> {
    let now = chrono::Local::now().timestamp_millis();
    let mut call = reqwest::Url::parse(
        "https://allertameteo.regione.emilia-romagna.it/o/api/allerta/get-sensor-values?variabile=254,0,0/1,-,-,-/B13215",
    )
    .unwrap();
    call.query_pairs_mut()
        .encoding_override(Some(&|s| s.as_bytes().into()))
        .append_pair("time", &now.to_string());
    let mut stations: Vec<Station> = reqwest::blocking::get(call)?.json::<Vec<_>>()?;
    stations.sort();
    if stations.is_empty() {
        return Ok(());
    }
    init_panic_hook();
    // setup terminal
    let mut terminal = init_tui()?;

    // create app and run it
    let app = App::<2>::new([
        Some(Page::Selection(SelectionPageState::new(stations))),
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
