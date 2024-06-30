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

use std::{default::Default, error::Error, io, ops::ControlFlow};

use allerta_meteo::{
    event_handler_trait::MutStatefulEventHandler,
    graph::{GraphPage, GraphPageState},
    table::{SelectionPage, SelectionPageState},
    Station,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Layout, Margin, Rect},
    style::{palette::tailwind, Color},
    terminal::{Frame, Terminal},
    widgets::{StatefulWidgetRef, WidgetRef},
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
    pages: [Page; N],
    page_idx: usize,
    colors: TableColors,
    color_index: usize,
}

impl<const N: usize> App<N> {
    pub fn new(pages: [Page; N]) -> Self {
        Self {
            pages,
            page_idx: 0,
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
        }
    }
    pub fn render(&mut self, frame: &mut Frame) {
        self.pages[self.page_idx].render(frame.size(), frame.buffer_mut());
    }
    pub fn handle_event(&mut self, event: Event) -> ControlFlow<(), ()> {
        match &mut self.pages[self.page_idx] {
            Page::Selection(state) => match (SelectionPage {}).handle(event, Some(state)) {
                ControlFlow::Continue(data) => {
                    if data.is_some() {
                        self.pages[1] = Page::Graph(GraphPageState::default());
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

fn main() -> Result<(), Box<dyn Error>> {
    let now = chrono::Local::now().timestamp_millis();
    let variabile = "254,0,0/1,-,-,-/B13215";
    let mut call = reqwest::Url::parse(
        "https://allertameteo.regione.emilia-romagna.it/o/api/allerta/get-sensor-values",
    )
    .unwrap();
    call.query_pairs_mut()
        .append_pair("variable", variabile)
        .append_pair("time", &now.to_string());
    let stations: Vec<Station> = reqwest::blocking::get(call)?.json::<Vec<_>>()?;
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::<2>::new([
        Page::Selection(SelectionPageState::new()),
        Page::Graph(GraphPageState::new()),
    ]);
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend, const N: usize>(
    terminal: &mut Terminal<B>,
    mut app: App<N>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| app.render(f))?;

        if let ControlFlow::Break(_) = app.handle_event(event::read()?) {
            return Ok(());
        }
    }
}