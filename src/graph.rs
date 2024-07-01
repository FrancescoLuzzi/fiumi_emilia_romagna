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

use crate::event_handler_trait::MutStatefulEventHandler;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::Span,
    widgets::{Axis, Block, Chart, Dataset, StatefulWidgetRef, Widget},
};
use std::{
    ops::ControlFlow,
    time::{Duration, Instant},
};

#[derive(Clone)]
struct SinSignal {
    x: f64,
    interval: f64,
    period: f64,
    scale: f64,
}

impl SinSignal {
    const fn new(interval: f64, period: f64, scale: f64) -> Self {
        Self {
            x: 0.0,
            interval,
            period,
            scale,
        }
    }
}

impl Iterator for SinSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, (self.x * 1.0 / self.period).sin() * self.scale);
        self.x += self.interval;
        Some(point)
    }
}

pub struct GraphPageState {
    signal1: SinSignal,
    signal2: SinSignal,
    data1: Vec<(f64, f64)>,
    data2: Vec<(f64, f64)>,
    window: [f64; 2],
    last_tick: Instant,
    tick_rate: Duration,
}

impl GraphPageState {
    pub fn new(period1: f64, period2: f64) -> Self {
        let mut signal1 = SinSignal::new(0.2, period1, 18.0);
        let mut signal2 = SinSignal::new(0.1, period2, 10.0);
        let data1 = signal1.by_ref().take(200).collect::<Vec<(f64, f64)>>();
        let data2 = signal2.by_ref().take(200).collect::<Vec<(f64, f64)>>();
        Self {
            signal1,
            data1,
            signal2,
            data2,
            window: [0.0, 20.0],
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl GraphPageState {
    fn on_tick(&mut self) {
        self.data1.drain(0..5);
        self.data1.extend(self.signal1.by_ref().take(5));

        self.data2.drain(0..10);
        self.data2.extend(self.signal2.by_ref().take(10));

        self.window[0] += 1.0;
        self.window[1] += 1.0;
    }
}

pub struct GraphPage {}
impl StatefulWidgetRef for GraphPage {
    type State = GraphPageState;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        if state.last_tick.elapsed() >= state.tick_rate {
            state.on_tick();
            state.last_tick = Instant::now();
        }
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
        let datasets = vec![
            Dataset::default()
                .name("data1")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Cyan))
                .data(&state.data1),
            Dataset::default()
                .name("data2")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Yellow))
                .data(&state.data2),
        ];

        let chart = Chart::new(datasets)
            .block(Block::bordered().title("Chart 1".cyan().bold()))
            .x_axis(
                Axis::default()
                    .title("X Axis")
                    .style(Style::default().fg(Color::Gray))
                    .labels(x_labels)
                    .bounds(state.window),
            )
            .y_axis(
                Axis::default()
                    .title("Y Axis")
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec!["-20".bold(), "0".into(), "20".bold()])
                    .bounds([-20.0, 20.0]),
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
