use crate::{
    config::UiConfig,
    graph::GraphPage,
    table::SelectionPage,
};
use alert_core::{
    api::{AlertClient, latest_station_time},
    model::{Station, Stations, TimeSeries},
};
use anyhow::Context;
use async_channel::{Receiver, Sender};
use chrono::TimeDelta;
use crossterm::event::Event;
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    buffer::Buffer,
    layout::Rect,
};
use std::time::Instant;

pub enum AppEvent {
    Input(Event),
    StationsLoaded(Stations),
    TimeSeriesLoaded {
        station_id: String,
        series: TimeSeries,
    },
    LoadFailed {
        scope: LoadScope,
        message: String,
    },
    Quit,
}

#[derive(Clone)]
pub enum LoadScope {
    Stations,
    TimeSeries { station_id: String },
}

pub enum UiAction {
    Redraw,
    Quit,
    OpenGraph { station: Station },
    BackToSelection,
}

enum Page {
    Selection(SelectionPage),
    Graph(GraphPage),
}

impl Page {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        match self {
            Page::Selection(page) => page.render(area, buf),
            Page::Graph(page) => page.render(area, buf),
        }
    }
}

pub struct App<const N: usize> {
    pages: [Option<Page>; N],
    page_idx: usize,
}

impl<const N: usize> App<N> {
    fn new(pages: [Option<Page>; N]) -> Self {
        Self { pages, page_idx: 0 }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        self.pages[self.page_idx]
            .as_mut()
            .expect("page should be initialized")
            .render(frame.area(), frame.buffer_mut());
    }

    fn current_page_mut(&mut self) -> &mut Page {
        self.pages[self.page_idx]
            .as_mut()
            .expect("page should be initialized")
    }

    fn selection_page_mut(&mut self) -> &mut SelectionPage {
        match self.pages[0].as_mut().expect("selection page should exist") {
            Page::Selection(page) => page,
            Page::Graph(_) => unreachable!("selection page index must contain selection state"),
        }
    }

    fn graph_page_mut(&mut self) -> Option<&mut GraphPage> {
        match self.pages[1].as_mut()? {
            Page::Graph(page) => Some(page),
            Page::Selection(_) => None,
        }
    }

    fn handle_action(&mut self, action: UiAction) -> AppReaction {
        match action {
            UiAction::Redraw => AppReaction {
                redraw: true,
                ..AppReaction::default()
            },
            UiAction::Quit => AppReaction {
                should_quit: true,
                redraw: true,
                ..AppReaction::default()
            },
            UiAction::OpenGraph { station } => {
                let station_id = station.idstazione().to_owned();
                self.pages[1] = Some(Page::Graph(GraphPage::loading(station)));
                self.page_idx = 1;
                AppReaction {
                    redraw: true,
                    load_request: Some(LoadScope::TimeSeries { station_id }),
                    ..AppReaction::default()
                }
            }
            UiAction::BackToSelection => {
                self.page_idx = 0;
                AppReaction {
                    redraw: true,
                    ..AppReaction::default()
                }
            }
        }
    }

    pub fn handle_event(&mut self, event: AppEvent) -> AppReaction {
        match event {
            AppEvent::Input(event) => {
                let action = match self.current_page_mut() {
                    Page::Selection(page) => page.handle_event(event),
                    Page::Graph(page) => page.handle_event(event),
                };
                self.handle_action(action)
            }
            AppEvent::StationsLoaded(mut stations) => {
                stations.sort_by_alert_desc();
                let selection = self.selection_page_mut();
                selection.set_error(None);
                selection.set_items(stations.into_vec());
                AppReaction {
                    redraw: true,
                    ..AppReaction::default()
                }
            }
            AppEvent::TimeSeriesLoaded { station_id, series } => {
                if let Some(graph) = self.graph_page_mut() {
                    if graph.station_id() == station_id {
                        graph.set_series(series);
                    }
                }
                AppReaction {
                    redraw: true,
                    ..AppReaction::default()
                }
            }
            AppEvent::LoadFailed { scope, message } => {
                match scope {
                    LoadScope::Stations => {
                        self.selection_page_mut().set_error(Some(message));
                    }
                    LoadScope::TimeSeries { station_id } => {
                        if let Some(graph) = self.graph_page_mut() {
                            if graph.station_id() == station_id {
                                graph.set_error(message);
                            }
                        }
                    }
                }
                AppReaction {
                    redraw: true,
                    ..AppReaction::default()
                }
            }
            AppEvent::Quit => AppReaction {
                should_quit: true,
                redraw: false,
                ..AppReaction::default()
            },
        }
    }
}

impl App<2> {
    pub fn with_default_pages() -> Self {
        Self::new([Some(Page::Selection(SelectionPage::new(Vec::new()))), None])
    }
}

#[derive(Default)]
pub struct AppReaction {
    pub redraw: bool,
    pub should_quit: bool,
    pub load_request: Option<LoadScope>,
}

pub async fn spawn_input_task(sender: Sender<AppEvent>) {
    std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(event) => {
                    if sender.send_blocking(AppEvent::Input(event)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = sender.send_blocking(AppEvent::Quit);
                    break;
                }
            }
        }
    });
}

fn visible_station_count(stations: &Stations) -> usize {
    stations
        .iter()
        .filter(|station| station.value().is_some())
        .count()
}

async fn load_initial_stations(client: AlertClient, sender: Sender<AppEvent>) {
    let mut now = match latest_station_time(chrono::Local::now()) {
        Ok(now) => now,
        Err(error) => {
            let _ = sender
                .send(AppEvent::LoadFailed {
                    scope: LoadScope::Stations,
                    message: error.to_string(),
                })
                .await;
            return;
        }
    };

    let delta_15_mins = TimeDelta::try_minutes(15).expect("15 minutes should be valid");

    loop {
        match client.stations_at(now).await {
            Ok(stations) if !stations.is_empty() && visible_station_count(&stations) >= 10 => {
                let _ = sender.send(AppEvent::StationsLoaded(stations)).await;
                return;
            }
            Ok(_) => {
                now -= delta_15_mins;
            }
            Err(error) => {
                let _ = sender
                    .send(AppEvent::LoadFailed {
                        scope: LoadScope::Stations,
                        message: error.to_string(),
                    })
                    .await;
                return;
            }
        }
    }
}

async fn load_timeseries(client: AlertClient, sender: Sender<AppEvent>, station_id: String) {
    match client.station_timeseries(&station_id).await {
        Ok(series) => {
            let _ = sender
                .send(AppEvent::TimeSeriesLoaded { station_id, series })
                .await;
        }
        Err(error) => {
            let _ = sender
                .send(AppEvent::LoadFailed {
                    scope: LoadScope::TimeSeries { station_id },
                    message: error.to_string(),
                })
                .await;
        }
    }
}

async fn apply_load_request(
    client: &AlertClient,
    sender: &Sender<AppEvent>,
    load_request: LoadScope,
) {
    match load_request {
        LoadScope::Stations => {
            tokio::spawn(load_initial_stations(client.clone(), sender.clone()));
        }
        LoadScope::TimeSeries { station_id } => {
            tokio::spawn(load_timeseries(client.clone(), sender.clone(), station_id));
        }
    }
}

async fn draw_if_due<B: Backend, const N: usize>(
    terminal: &mut Terminal<B>,
    app: &mut App<N>,
    dirty: &mut bool,
    last_draw_at: &mut Instant,
    config: UiConfig,
) -> anyhow::Result<()> {
    if !*dirty {
        return Ok(());
    }

    let frame_interval = config.frame_interval();
    let elapsed = last_draw_at.elapsed();
    if elapsed < frame_interval {
        tokio::time::sleep(frame_interval - elapsed).await;
    }

    terminal
        .draw(|frame| app.render(frame))
        .map_err(|err| anyhow::anyhow!("Error drawing frame: {err}"))?;
    *last_draw_at = Instant::now();
    *dirty = false;
    Ok(())
}

pub async fn run_app<B: Backend, const N: usize>(
    terminal: &mut Terminal<B>,
    mut app: App<N>,
    config: UiConfig,
    receiver: Receiver<AppEvent>,
    sender: Sender<AppEvent>,
    client: AlertClient,
) -> anyhow::Result<()> {
    let mut dirty = true;
    let mut last_draw_at = Instant::now() - config.frame_interval();

    draw_if_due(terminal, &mut app, &mut dirty, &mut last_draw_at, config).await?;

    loop {
        let first_event = receiver.recv().await.context("event channel closed")?;
        let mut reaction = app.handle_event(first_event);
        dirty |= reaction.redraw;
        if let Some(load_request) = reaction.load_request.take() {
            apply_load_request(&client, &sender, load_request).await;
        }
        if reaction.should_quit {
            return Ok(());
        }

        for _ in 1..config.max_events_per_batch {
            let Ok(event) = receiver.try_recv() else {
                break;
            };
            reaction = app.handle_event(event);
            dirty |= reaction.redraw;
            if let Some(load_request) = reaction.load_request.take() {
                apply_load_request(&client, &sender, load_request).await;
            }
            if reaction.should_quit {
                return Ok(());
            }
        }

        draw_if_due(terminal, &mut app, &mut dirty, &mut last_draw_at, config).await?;
    }
}

pub async fn bootstrap() -> (App<2>, AlertClient, Sender<AppEvent>, Receiver<AppEvent>) {
    let client = AlertClient::new();
    let (sender, receiver) = async_channel::bounded::<AppEvent>(256);

    spawn_input_task(sender.clone()).await;
    tokio::spawn(load_initial_stations(client.clone(), sender.clone()));

    (App::<2>::with_default_pages(), client, sender, receiver)
}
