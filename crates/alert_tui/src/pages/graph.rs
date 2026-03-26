use crate::framework::{PageModel, RenderablePageModel, Task, Update};
use alert_core::{
    api::AlertClient,
    model::{Station, TimeSeries},
};
use chrono::{Local, TimeZone};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::HorizontalAlignment,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::Span,
    widgets::{Axis, Block, Chart, Dataset, Paragraph, Widget},
};

const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

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
    soglia1_label: String,
    soglia2_label: String,
    soglia3_label: String,
    window: [f64; 2],
}

pub enum Action {
    Back,
}

pub enum Message {
    TimeSeriesLoaded(TimeSeries),
    LoadFailed(String),
}

impl GraphPage {
    pub fn loading(station: Station) -> Self {
        let soglia1_label = format!("Soglia1 ({})", station.soglia1());
        let soglia2_label = format!("Soglia2 ({})", station.soglia2());
        let soglia3_label = format!("Soglia3 ({})", station.soglia3());
        Self {
            soglia1_data: Vec::new(),
            soglia2_data: Vec::new(),
            soglia3_data: Vec::new(),
            station,
            soglia1_label,
            soglia2_label,
            soglia3_label,
            data_state: GraphDataState::Loading,
            window: [0.0, 1.0],
        }
    }

    pub fn set_series(&mut self, data: TimeSeries) {
        let data = data.as_dataset();
        self.soglia1_data = data
            .iter()
            .map(|(timestamp, _)| (*timestamp, *self.station.soglia1() as f64))
            .collect();
        self.soglia2_data = data
            .iter()
            .map(|(timestamp, _)| (*timestamp, *self.station.soglia2() as f64))
            .collect();
        self.soglia3_data = data
            .iter()
            .map(|(timestamp, _)| (*timestamp, *self.station.soglia3() as f64))
            .collect();

        self.window = match (data.first(), data.last()) {
            (Some(first), Some(last)) if first.0 < last.0 => [first.0, last.0],
            (Some(first), Some(_)) => [first.0, first.0 + 1.0],
            _ => [0.0, 1.0],
        };
        self.data_state = GraphDataState::Ready { data };
    }

    pub fn set_error(&mut self, error: String) {
        self.data_state = GraphDataState::Error(error);
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
                let x_labels = build_x_labels(self.window);

                let datasets = vec![
                    Dataset::default()
                        .name("Rilevazione")
                        .marker(symbols::Marker::Dot)
                        .style(Style::default().fg(Color::Cyan))
                        .data(data),
                    Dataset::default()
                        .name(self.soglia1_label.as_str())
                        .marker(symbols::Marker::Braille)
                        .style(Style::default().fg(Color::Green))
                        .data(&self.soglia1_data),
                    Dataset::default()
                        .name(self.soglia2_label.as_str())
                        .marker(symbols::Marker::Braille)
                        .style(Style::default().fg(Color::Yellow))
                        .data(&self.soglia2_data),
                    Dataset::default()
                        .name(self.soglia3_label.as_str())
                        .marker(symbols::Marker::Braille)
                        .style(Style::default().fg(Color::Red))
                        .data(&self.soglia3_data),
                ];

                Chart::new(datasets)
                    .block(Block::bordered().title(self.station.nomestaz().cyan().bold()))
                    .x_axis(
                        Axis::default()
                            .title("Time (local)")
                            .style(Style::default().fg(Color::Gray))
                            .labels(x_labels)
                            .labels_alignment(HorizontalAlignment::Right)
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

impl PageModel for GraphPage {
    type Action = Action;
    type Message = Message;

    fn init(&mut self) -> Update<Self::Action, Self::Message> {
        Update::task(Task::perform(
            load_timeseries(self.station.idstazione().to_owned()),
            |result| match result {
                Ok(series) => Message::TimeSeriesLoaded(series),
                Err(message) => Message::LoadFailed(message),
            },
        ))
    }

    fn handle_event(&mut self, event: Event) -> Update<Self::Action, Self::Message> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                return match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => Update::action(Action::Back),
                    _ => Update::none(),
                };
            }
        }

        Update::none()
    }

    fn update(&mut self, message: Self::Message) -> Update<Self::Action, Self::Message> {
        match message {
            Message::TimeSeriesLoaded(series) => {
                self.set_series(series);
                Update::redraw()
            }
            Message::LoadFailed(message) => {
                self.set_error(message);
                Update::redraw()
            }
        }
    }
}

impl RenderablePageModel for GraphPage {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        GraphPage::render(self, area, buf);
    }
}

async fn load_timeseries(station_id: String) -> Result<TimeSeries, String> {
    let client = AlertClient::new();
    client
        .station_timeseries(&station_id)
        .await
        .map_err(|error| error.to_string())
}

fn build_x_labels(window: [f64; 2]) -> Vec<Span<'static>> {
    let start = window[0].round() as i64;
    let end = window[1].round() as i64;

    if start >= end {
        return vec![Span::styled(
            format_timestamp_label(start),
            Style::default().blue().add_modifier(Modifier::BOLD),
        )];
    }

    let span = end.saturating_sub(start);
    let segments = ((span + MILLIS_PER_DAY - 1) / MILLIS_PER_DAY).max(1) as usize;
    let mut labels = Vec::with_capacity(segments + 1);

    for index in 0..=segments {
        let timestamp = if index == segments {
            end
        } else {
            start + ((span as i128 * index as i128) / segments as i128) as i64
        };

        let label = format_timestamp_label(timestamp);
        let span = if index == 0 {
            Span::styled(label, Style::default().blue().add_modifier(Modifier::BOLD))
        } else if index == segments {
            Span::styled(
                label,
                Style::default().yellow().add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw(label)
        };

        labels.push(span);
    }

    labels
}

fn format_timestamp_label(timestamp_ms: i64) -> String {
    Local
        .timestamp_millis_opt(timestamp_ms)
        .single()
        .map(|time| time.format("%d %b %H:%M").to_string())
        .unwrap_or_else(|| timestamp_ms.to_string())
}
