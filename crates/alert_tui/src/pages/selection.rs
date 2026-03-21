use crate::framework::{PageModel, RenderablePageModel, Task, Update};
use alert_core::{
    api::{AlertClient, latest_station_time},
    model::{Station, Stations},
};
use chrono::TimeDelta;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use frizbee::{Config, match_list};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    style::palette::tailwind,
    text::{Line, Text},
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Table, TableState,
    },
};
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

const FILTER_DEBOUNCE_TASK: &str = "selection/filter_debounce";

const INFO_TEXT: &str = "(q) quit | (/) filter | (↑) move up | (↓) move down | (Enter) see graph";
const FILTER_INFO_TEXT: &str =
    "(Esc) exit filter | (Ctrl+C/Ctrl+Bksp) clear | (↑/↓) move | (Enter) see graph";
const ITEM_HEIGHT: usize = 4;

pub struct SelectionPage {
    table_state: TableState,
    all_items: Vec<Station>,
    items: Vec<Station>,
    longest_item_lens: (u16, u16, u16, u16, u16),
    scroll_state: ScrollbarState,
    error: Option<String>,
    filter_mode: bool,
    filter_query: String,
    filter_debounce_delay: Duration,
}

pub enum Action {
    Quit,
    OpenGraph { station: Station },
}

pub enum Message {
    ApplyFilter,
    StationsLoaded(Stations),
    LoadFailed(String),
}

impl SelectionPage {
    pub fn new(items: Vec<Station>, filter_debounce_delay: Duration) -> Self {
        let mut page = Self {
            table_state: TableState::default().with_selected(0),
            longest_item_lens: (0, 0, 0, 0, 0),
            scroll_state: ScrollbarState::new(0),
            all_items: items,
            items: Vec::new(),
            error: None,
            filter_mode: false,
            filter_query: String::new(),
            filter_debounce_delay,
        };
        page.apply_filter();
        page
    }

    pub fn set_items(&mut self, items: Vec<Station>) {
        self.all_items = items;
        self.apply_filter();
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }

    fn handle_normal_mode_key(&mut self, key: KeyEvent) -> Update<Action, Message> {
        match key.code {
            KeyCode::Char('/') => {
                self.filter_mode = true;
                Update::redraw()
            }
            KeyCode::Char('q') | KeyCode::Esc => Update::action(Action::Quit),
            KeyCode::Char('j') | KeyCode::Down => {
                self.next();
                Update::redraw()
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.previous();
                Update::redraw()
            }
            KeyCode::Enter => self
                .selected_station()
                .map(|station| Update::action(Action::OpenGraph { station }))
                .unwrap_or_else(Update::none),
            _ => Update::none(),
        }
    }

    fn handle_filter_mode_key(&mut self, key: KeyEvent) -> Update<Action, Message> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') | KeyCode::Backspace => self.clear_filter(),
                _ => Update::none(),
            };
        }

        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.filter_mode = false;
                Update::redraw()
            }
            KeyCode::Backspace => {
                if self.filter_query.pop().is_some() {
                    self.request_filter_update()
                } else {
                    Update::none()
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.filter_query.push(c);
                self.request_filter_update()
            }
            _ => Update::none(),
        }
    }

    fn request_filter_update(&mut self) -> Update<Action, Message> {
        let delay = self.filter_debounce_delay;
        Update::task(Task::keyed(FILTER_DEBOUNCE_TASK, async move {
            tokio::time::sleep(delay).await;
            Message::ApplyFilter
        }))
        .and_redraw()
    }

    fn clear_filter(&mut self) -> Update<Action, Message> {
        if self.filter_query.is_empty() {
            return Update::none();
        }

        self.filter_query.clear();
        self.request_filter_update()
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);
        self.render_table(buf, rects[0]);
        self.render_scrollbar(buf, rects[0]);
        render_footer(
            buf,
            rects[1],
            self.error.as_deref(),
            &self.filter_query,
            self.filter_mode,
        );
    }

    fn apply_filter(&mut self) {
        let selected_id = self
            .selected_station()
            .map(|station| station.idstazione().to_owned());
        let filtered_items = if self.filter_query.is_empty() {
            self.all_items.clone()
        } else {
            let haystacks = self
                .all_items
                .iter()
                .map(Station::nomestaz)
                .collect::<Vec<_>>();
            let matches = match_list(&self.filter_query, &haystacks, &Config::default());

            matches
                .into_iter()
                .filter_map(|matched| self.all_items.get(matched.index as usize).cloned())
                .collect::<Vec<_>>()
        };

        self.set_visible_items(filtered_items, selected_id.as_deref());
    }

    fn set_visible_items(&mut self, items: Vec<Station>, selected_id: Option<&str>) {
        self.longest_item_lens = constraint_len_calculator(&items);
        self.scroll_state =
            ScrollbarState::new((items.len().saturating_sub(1)).saturating_mul(ITEM_HEIGHT));
        self.items = items;

        let selected = selected_id
            .and_then(|id| {
                self.items
                    .iter()
                    .position(|station| station.idstazione() == id)
            })
            .or_else(|| (!self.items.is_empty()).then_some(0));

        self.table_state.select(selected);
        if let Some(selected) = selected {
            self.scroll_state = self.scroll_state.position(selected * ITEM_HEIGHT);
        }
    }

    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) if i >= self.items.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(0) => self.items.len() - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    fn selected_station(&self) -> Option<Station> {
        self.table_state
            .selected()
            .and_then(|i| self.items.get(i))
            .cloned()
    }

    fn render_table(&mut self, buf: &mut Buffer, area: Rect) {
        let header = [
            "Stazione",
            "Ultima rilevazione",
            "Soglia1",
            "Soglia2",
            "Soglia3",
        ]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .height(1);
        let rows = self.items.iter().map(|data| {
            let item = station_row(data);
            let style = match data.value().unwrap_or(&f32::MIN) {
                x if *x > *data.soglia3() => tailwind::VIOLET.c500,
                x if *x > *data.soglia2() => tailwind::RED.c500,
                x if *x > *data.soglia1() => tailwind::YELLOW.c500,
                _ => tailwind::GREEN.c500,
            };
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .height(ITEM_HEIGHT as u16)
                .style(style)
        });
        let bar = " █ ";
        Table::new(
            rows,
            [
                Constraint::Length(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.1 + 1),
                Constraint::Min(self.longest_item_lens.2 + 1),
                Constraint::Min(self.longest_item_lens.3 + 1),
                Constraint::Min(self.longest_item_lens.4),
            ],
        )
        .header(header)
        .highlight_symbol(Text::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .highlight_spacing(HighlightSpacing::Always)
        .render(area, buf, &mut self.table_state);
    }

    fn render_scrollbar(&mut self, buf: &mut Buffer, area: Rect) {
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .render(
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 1,
                }),
                buf,
                &mut self.scroll_state,
            )
    }
}

impl PageModel for SelectionPage {
    type Action = Action;
    type Message = Message;

    fn init(&mut self) -> Update<Self::Action, Self::Message> {
        Update::task(Task::perform(
            load_initial_stations(),
            |result| match result {
                Ok(stations) => Message::StationsLoaded(stations),
                Err(message) => Message::LoadFailed(message),
            },
        ))
    }

    fn handle_event(&mut self, event: Event) -> Update<Self::Action, Self::Message> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                return if self.filter_mode {
                    self.handle_filter_mode_key(key)
                } else {
                    self.handle_normal_mode_key(key)
                };
            }
        }

        Update::none()
    }

    fn update(&mut self, message: Self::Message) -> Update<Self::Action, Self::Message> {
        match message {
            Message::ApplyFilter => {
                self.apply_filter();
                Update::redraw()
            }
            Message::StationsLoaded(mut stations) => {
                stations.sort_by_alert_desc();
                self.set_error(None);
                self.set_items(stations.into_vec());
                Update::redraw()
            }
            Message::LoadFailed(message) => {
                self.set_error(Some(message));
                Update::redraw()
            }
        }
    }
}

impl RenderablePageModel for SelectionPage {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        SelectionPage::render(self, area, buf);
    }
}

fn visible_station_count(stations: &Stations) -> usize {
    stations
        .iter()
        .filter(|station| station.value().is_some())
        .count()
}

async fn load_initial_stations() -> Result<Stations, String> {
    let client = AlertClient::new();
    let mut now = latest_station_time(chrono::Local::now()).map_err(|error| error.to_string())?;
    let delta_15_mins = TimeDelta::try_minutes(15).expect("15 minutes should be valid");

    loop {
        match client.stations_at(now).await {
            Ok(stations) if !stations.is_empty() && visible_station_count(&stations) >= 10 => {
                return Ok(stations);
            }
            Ok(_) => {
                now -= delta_15_mins;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn station_row(data: &Station) -> [String; 5] {
    [
        data.nomestaz().to_owned(),
        data.value().copied().unwrap_or(0.0).to_string(),
        data.soglia1().to_string(),
        data.soglia2().to_string(),
        data.soglia3().to_string(),
    ]
}

fn constraint_len_calculator(items: &[Station]) -> (u16, u16, u16, u16, u16) {
    let nomestaz_len = items
        .iter()
        .map(Station::nomestaz)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let value_len = items
        .iter()
        .filter_map(Station::value)
        .map(f32::to_string)
        .map(|x| UnicodeWidthStr::width(x.as_str()))
        .max()
        .unwrap_or(0);
    let soglia1_len = items
        .iter()
        .map(Station::soglia1)
        .map(f32::to_string)
        .map(|x| UnicodeWidthStr::width(x.as_str()))
        .max()
        .unwrap_or(0);
    let soglia2_len = items
        .iter()
        .map(Station::soglia2)
        .map(f32::to_string)
        .map(|x| UnicodeWidthStr::width(x.as_str()))
        .max()
        .unwrap_or(0);
    let soglia3_len = items
        .iter()
        .map(Station::soglia3)
        .map(f32::to_string)
        .map(|x| UnicodeWidthStr::width(x.as_str()))
        .max()
        .unwrap_or(0);

    (
        nomestaz_len as u16,
        value_len as u16,
        soglia1_len as u16,
        soglia2_len as u16,
        soglia3_len as u16,
    )
}

fn render_footer(
    buf: &mut Buffer,
    area: Rect,
    error: Option<&str>,
    filter_query: &str,
    filter_mode: bool,
) {
    let filter_label = if filter_query.is_empty() {
        "Filter: off".to_owned()
    } else if filter_mode {
        format!("Filter> {filter_query}")
    } else {
        format!("Filter: {filter_query}")
    };

    let help = if filter_mode {
        FILTER_INFO_TEXT
    } else {
        INFO_TEXT
    };

    let text = match error {
        Some(error) => format!("{filter_label} | {help} | Error: {error}"),
        None => format!("{filter_label} | {help}"),
    };

    let footer =
        Paragraph::new(Line::from(text)).block(Block::bordered().border_type(BorderType::Double));
    ratatui::widgets::Widget::render(footer, area, buf);
}
