use crate::framework::{PageModel, RenderablePageModel, Task, Update};
use alert_core::{
    api::AlertClient,
    model::{Station, TimeSeries},
};
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

pub enum Action {
    Back,
}

pub enum Message {
    TimeSeriesLoaded(TimeSeries),
    LoadFailed(String),
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
