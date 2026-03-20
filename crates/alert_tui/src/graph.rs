use crate::app::UiAction;
use alert_core::model::{Station, TimeSeries};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::Span,
    widgets::{Axis, Block, Chart, Dataset, Paragraph, Widget},
};

pub enum GraphDataState {
    Loading,
    Ready { data: Vec<(f64, f64)> },
    Error(String),
}

pub struct GraphPage {
    station: Station,
    data_state: GraphDataState,
    soglia1_data: Vec<(f64, f64)>,
    soglia2_data: Vec<(f64, f64)>,
    soglia3_data: Vec<(f64, f64)>,
    window: [f64; 2],
}

impl GraphPage {
    pub fn loading(station: Station) -> Self {
        Self {
            soglia1_data: Vec::new(),
            soglia2_data: Vec::new(),
            soglia3_data: Vec::new(),
            station,
            data_state: GraphDataState::Loading,
            window: [0.0, 1.0],
        }
    }

    pub fn set_series(&mut self, data: TimeSeries) {
        let size = data.len().max(1) as f64;
        let data = data.as_dataset();
        self.soglia1_data = (0..data.len())
            .map(|i| (i as f64, *self.station.soglia1() as f64))
            .collect();
        self.soglia2_data = (0..data.len())
            .map(|i| (i as f64, *self.station.soglia2() as f64))
            .collect();
        self.soglia3_data = (0..data.len())
            .map(|i| (i as f64, *self.station.soglia3() as f64))
            .collect();
        self.window = [0.0, size];
        self.data_state = GraphDataState::Ready { data };
    }

    pub fn set_error(&mut self, error: String) {
        self.data_state = GraphDataState::Error(error);
    }

    pub fn station_id(&self) -> &str {
        self.station.idstazione()
    }

    pub fn handle_event(&mut self, event: Event) -> UiAction {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                return match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => UiAction::BackToSelection,
                    _ => UiAction::Redraw,
                };
            }
        }

        UiAction::Redraw
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        match &self.data_state {
            GraphDataState::Loading => {
                Paragraph::new("Loading graph data...")
                    .centered()
                    .block(Block::bordered().title(self.station.nomestaz().cyan().bold()))
                    .render(area, buf);
            }
            GraphDataState::Error(error) => {
                Paragraph::new(error.as_str())
                    .centered()
                    .block(Block::bordered().title(self.station.nomestaz().red().bold()))
                    .render(area, buf);
            }
            GraphDataState::Ready { data } => {
                let x_labels = vec![
                    Span::styled(
                        format!("{}", self.window[0]),
                        Style::default().blue().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("{}", (self.window[0] + self.window[1]) / 2.0)),
                    Span::styled(
                        format!("{}", self.window[1]),
                        Style::default().yellow().add_modifier(Modifier::BOLD),
                    ),
                ];

                let datasets = vec![
                    Dataset::default()
                        .name("Rilevazione")
                        .marker(symbols::Marker::Dot)
                        .style(Style::default().fg(Color::Cyan))
                        .data(data),
                    Dataset::default()
                        .name("Soglia1")
                        .marker(symbols::Marker::Braille)
                        .style(Style::default().fg(Color::Green))
                        .data(&self.soglia1_data),
                    Dataset::default()
                        .name("Soglia2")
                        .marker(symbols::Marker::Braille)
                        .style(Style::default().fg(Color::Yellow))
                        .data(&self.soglia2_data),
                    Dataset::default()
                        .name("Soglia3")
                        .marker(symbols::Marker::Braille)
                        .style(Style::default().fg(Color::Red))
                        .data(&self.soglia3_data),
                ];

                Chart::new(datasets)
                    .block(Block::bordered().title(self.station.nomestaz().cyan().bold()))
                    .x_axis(
                        Axis::default()
                            .title("Time")
                            .style(Style::default().fg(Color::Gray))
                            .labels(x_labels)
                            .bounds(self.window),
                    )
                    .y_axis(
                        Axis::default()
                            .title("Rilevazione")
                            .style(Style::default().fg(Color::Gray))
                            .labels(vec!["-20".bold(), "0".into(), "20".bold()])
                            .bounds([-20.0, 40.0]),
                    )
                    .render(area, buf);
            }
        }
    }
}
