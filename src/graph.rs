//! # [Ratatui] Chart example
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

use crate::{event_handler_trait::MutStatefulEventHandler, Station, TimeSeries};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::Span,
    widgets::{Axis, Block, Chart, Dataset, StatefulWidgetRef, Widget},
};
use std::ops::ControlFlow;

pub struct GraphPageState {
    station: Station,
    data: Vec<(f64, f64)>,

    window: [f64; 2],
}

impl GraphPageState {
    pub fn new(station: Station, data: TimeSeries) -> Self {
        let size = data.0.len() as f64;
        Self {
            station,
            data: data.as_dataset(),
            window: [0.0, size],
        }
    }
}

pub struct GraphPage {}
impl StatefulWidgetRef for GraphPage {
    type State = GraphPageState;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let x_labels = vec![
            Span::styled(
                format!("{}", state.window[0]),
                Style::default().blue().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}", (state.window[0] + state.window[1]) / 2.0)),
            Span::styled(
                format!("{}", state.window[1]),
                Style::default().yellow().add_modifier(Modifier::BOLD),
            ),
        ];
        let datasets = vec![Dataset::default()
            .name("data1")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Cyan))
            .data(&state.data)];

        let chart = Chart::new(datasets)
            .block(Block::bordered().title(state.station.nomestaz().cyan().bold()))
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().fg(Color::Gray))
                    .labels(x_labels)
                    .bounds(state.window),
            )
            .y_axis(
                Axis::default()
                    .title("Rilevazione")
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec!["-20".bold(), "0".into(), "20".bold()])
                    .bounds([-20.0, 40.0]),
            );

        chart.render(area, buf)
    }
}

impl MutStatefulEventHandler<GraphPageState, (), ()> for GraphPage {
    fn handle(
        &mut self,
        event: crossterm::event::Event,
        _state: Option<&mut GraphPageState>,
    ) -> ControlFlow<(), ()> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                return match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => ControlFlow::Break(()),
                    _ => ControlFlow::Continue(()),
                };
            }
        }
        ControlFlow::Continue(())
    }
}
