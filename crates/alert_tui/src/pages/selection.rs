use crate::framework::{PageModel, RenderablePageModel, Task, Update};
use alert_core::{
    api::{AlertClient, DELTA_15MIN, clamp_station_time, latest_station_time},
    model::{Station, Stations},
};
use chrono::{DateTime, Local, LocalResult, NaiveDateTime, TimeDelta, TimeZone};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use frizbee::{Config, match_list};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Style, palette::tailwind},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Cell, Clear, HighlightSpacing, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget, Table, TableState,
    },
};
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

const FILTER_DEBOUNCE_TASK: &str = "selection/filter_debounce";
const LOAD_STATIONS_TASK: &str = "selection/load_stations";
const QUERY_TIME_FORMAT: &str = "%Y-%m-%d %H:%M";

const INFO_TEXT: &str = "(q) quit | (/) filter | (t) set time | (←/→) +/-15m | (n) latest | (↑/↓) move | (Enter) see graph";
const FILTER_INFO_TEXT: &str =
    "(Esc) exit filter | (Ctrl+C/Ctrl+Bksp) clear | (↑/↓) move | (Enter) see graph";
const QUERY_INFO_TEXT: &str =
    "(Esc) cancel | (Enter) load time | format YYYY-MM-DD HH:MM | max now";
const ITEM_HEIGHT: usize = 4;

#[derive(Clone, Copy)]
enum SelectionPageState {
    Normal,
    Filter,
    Query,
}

pub struct LoadedPageData {
    stations: Stations,
    resolved_time: DateTime<Local>,
}

enum SelectionPageData {
    Loading,
    Loaded(LoadedPageData),
}

pub struct SelectionPage {
    table_state: TableState,
    state: SelectionPageState,
    data: SelectionPageData,
    items: Vec<Station>,
    longest_item_lens: (u16, u16, u16, u16, u16),
    scroll_state: ScrollbarState,
    error: Option<String>,
    filter_query: String,
    time_input: String,
    time_input_cursor: usize,
    stations_request_inflight: bool,
    filter_debounce_delay: Duration,
}

pub enum Action {
    Quit,
    OpenGraph { station: Station },
}

pub enum Message {
    ApplyFilter,
    StationsLoaded(LoadedPageData),
    LoadFailed(String),
}

impl SelectionPage {
    pub fn new(filter_debounce_delay: Duration) -> Self {
        Self {
            table_state: TableState::default().with_selected(0),
            state: SelectionPageState::Normal,
            longest_item_lens: (0, 0, 0, 0, 0),
            scroll_state: ScrollbarState::new(0),
            data: SelectionPageData::Loading,
            items: Vec::new(),
            error: None,
            filter_query: String::new(),
            time_input: String::new(),
            time_input_cursor: 0,
            stations_request_inflight: false,
            filter_debounce_delay,
        }
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }

    fn loaded_data(&self) -> Option<&LoadedPageData> {
        match &self.data {
            SelectionPageData::Loading => None,
            SelectionPageData::Loaded(data) => Some(data),
        }
    }

    fn loaded_time(&self) -> Option<DateTime<Local>> {
        self.loaded_data().map(|data| data.resolved_time)
    }

    fn enter_filter_mode(&mut self) -> Update<Action, Message> {
        self.state = SelectionPageState::Filter;
        Update::redraw()
    }

    fn exit_filter_mode(&mut self) -> Update<Action, Message> {
        self.state = SelectionPageState::Normal;
        Update::redraw()
    }

    fn enter_query_mode(&mut self) -> Update<Action, Message> {
        let Some(time) = self.loaded_time() else {
            return Update::none();
        };

        self.state = SelectionPageState::Query;
        self.time_input = format_time(time);
        self.time_input_cursor = self.time_input.len();
        Update::redraw()
    }

    fn exit_query_mode(&mut self) -> Update<Action, Message> {
        self.state = SelectionPageState::Normal;
        self.time_input = self.loaded_time().map(format_time).unwrap_or_default();
        self.time_input_cursor = self.time_input.len();
        Update::redraw()
    }

    fn handle_normal_mode_key(&mut self, key: KeyEvent) -> Update<Action, Message> {
        match key.code {
            KeyCode::Char('/') => self.enter_filter_mode(),
            KeyCode::Char('t') => self.enter_query_mode(),
            KeyCode::Left | KeyCode::Char('h') => self.shift_query_time(-DELTA_15MIN),
            KeyCode::Right | KeyCode::Char('l') => self.shift_query_time(DELTA_15MIN),
            KeyCode::Char('n') => self.jump_to_latest_query_time(),
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

    fn handle_query_mode_key(&mut self, key: KeyEvent) -> Update<Action, Message> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') | KeyCode::Backspace => self.clear_query_input(),
                _ => Update::none(),
            };
        }

        match key.code {
            KeyCode::Esc => self.exit_query_mode(),
            KeyCode::Enter => match parse_time_input(&self.time_input) {
                Ok(requested_time) => {
                    self.state = SelectionPageState::Normal;
                    self.set_error(None);
                    self.load_time(clamp_time_to_latest(requested_time))
                }
                Err(message) => {
                    self.set_error(Some(message));
                    Update::redraw()
                }
            },
            KeyCode::Left => self.move_time_cursor_left(),
            KeyCode::Right => self.move_time_cursor_right(),
            KeyCode::Home => self.move_time_cursor_to_start(),
            KeyCode::End => self.move_time_cursor_to_end(),
            KeyCode::Backspace => self.backspace_time_input(),
            KeyCode::Delete => self.delete_time_input(),
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.insert_time_input(c)
            }
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
            KeyCode::Esc | KeyCode::Enter => self.exit_filter_mode(),
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

    fn clear_query_input(&mut self) -> Update<Action, Message> {
        if self.time_input.is_empty() {
            return Update::none();
        }

        self.time_input.clear();
        self.time_input_cursor = 0;
        Update::redraw()
    }

    fn move_time_cursor_left(&mut self) -> Update<Action, Message> {
        if self.time_input_cursor == 0 {
            return Update::none();
        }

        self.time_input_cursor -= 1;
        Update::redraw()
    }

    fn move_time_cursor_right(&mut self) -> Update<Action, Message> {
        if self.time_input_cursor >= self.time_input.len() {
            return Update::none();
        }

        self.time_input_cursor += 1;
        Update::redraw()
    }

    fn move_time_cursor_to_start(&mut self) -> Update<Action, Message> {
        if self.time_input_cursor == 0 {
            return Update::none();
        }

        self.time_input_cursor = 0;
        Update::redraw()
    }

    fn move_time_cursor_to_end(&mut self) -> Update<Action, Message> {
        let end = self.time_input.len();
        if self.time_input_cursor == end {
            return Update::none();
        }

        self.time_input_cursor = end;
        Update::redraw()
    }

    fn backspace_time_input(&mut self) -> Update<Action, Message> {
        if self.time_input_cursor == 0 {
            return Update::none();
        }

        self.time_input_cursor -= 1;
        self.time_input.remove(self.time_input_cursor);
        Update::redraw()
    }

    fn delete_time_input(&mut self) -> Update<Action, Message> {
        if self.time_input_cursor >= self.time_input.len() {
            return Update::none();
        }

        self.time_input.remove(self.time_input_cursor);
        Update::redraw()
    }

    fn insert_time_input(&mut self, c: char) -> Update<Action, Message> {
        self.time_input.insert(self.time_input_cursor, c);
        self.time_input_cursor += c.len_utf8();
        Update::redraw()
    }

    fn shift_query_time(&mut self, delta: TimeDelta) -> Update<Action, Message> {
        let Some(current_time) = self.loaded_time() else {
            return Update::none();
        };

        let next_time = clamp_time_to_latest(current_time + delta);
        if next_time == current_time {
            return Update::none();
        }

        self.set_error(None);
        self.load_time(next_time)
    }

    fn jump_to_latest_query_time(&mut self) -> Update<Action, Message> {
        self.set_error(None);
        self.load_time(latest_station_time().unwrap())
    }

    fn load_time(&mut self, requested_time: DateTime<Local>) -> Update<Action, Message> {
        if self.stations_request_inflight {
            return Update::none();
        }
        self.request_stations_load(requested_time)
    }

    fn request_stations_load(
        &mut self,
        requested_time: DateTime<Local>,
    ) -> Update<Action, Message> {
        if self.stations_request_inflight {
            return Update::none();
        }

        self.stations_request_inflight = true;
        Update::task(Task::keyed(LOAD_STATIONS_TASK, async move {
            match load_page_data(requested_time).await {
                Ok(data) => Message::StationsLoaded(data),
                Err(message) => Message::LoadFailed(message),
            }
        }))
        .and_redraw()
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(4)]).split(area);
        self.render_table(buf, rects[0]);
        self.render_scrollbar(buf, rects[0]);
        render_footer(
            buf,
            rects[1],
            self.error.as_deref(),
            self.loaded_data(),
            &self.time_input,
            &self.filter_query,
            self.state,
        );

        if matches!(self.state, SelectionPageState::Query) {
            dim_area(buf, area);
            render_time_popup(buf, area, &self.time_input, self.error.as_deref());
        }
    }

    fn cursor_position(&self, area: Rect) -> Option<(u16, u16)> {
        if !matches!(self.state, SelectionPageState::Query) {
            return None;
        }

        let popup_area = centered_rect(area, 64, 7);
        let input_x = popup_area.x.saturating_add(7);
        let input_y = popup_area.y.saturating_add(1);
        let max_x = popup_area.right().saturating_sub(2);
        let cursor_x = input_x
            .saturating_add(self.time_input[..self.time_input_cursor].width() as u16)
            .min(max_x);

        Some((cursor_x, input_y))
    }

    fn apply_filter(&mut self) {
        let selected_id = self
            .selected_station()
            .map(|station| station.idstazione().to_owned());
        let filtered_items = self
            .loaded_data()
            .map(|data| filter_stations(&data.stations, &self.filter_query))
            .unwrap_or_default();

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
        let rows = self.items.iter().map(station_row);
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
        self.jump_to_latest_query_time()
    }

    fn handle_event(&mut self, event: Event) -> Update<Self::Action, Self::Message> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                return match self.state {
                    SelectionPageState::Normal => self.handle_normal_mode_key(key),
                    SelectionPageState::Filter => self.handle_filter_mode_key(key),
                    SelectionPageState::Query => self.handle_query_mode_key(key),
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
            Message::StationsLoaded(mut data) => {
                self.stations_request_inflight = false;
                data.stations.sort_by_alert_desc();
                self.data = SelectionPageData::Loaded(data);
                self.set_error(None);
                self.apply_filter();
                Update::redraw()
            }
            Message::LoadFailed(message) => {
                self.stations_request_inflight = false;
                self.set_error(Some(message));
                Update::redraw()
            }
        }
    }
}

impl RenderablePageModel for SelectionPage {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.render(area, buf);
    }

    fn cursor_position(&self, area: Rect) -> Option<(u16, u16)> {
        self.cursor_position(area)
    }
}

fn visible_station_count(stations: &Stations) -> usize {
    stations
        .iter()
        .filter(|station| station.value().is_some())
        .count()
}

async fn load_page_data(requested_time: DateTime<Local>) -> Result<LoadedPageData, String> {
    let client = AlertClient::new();
    let mut resolved_time = clamp_time_to_latest(requested_time);

    loop {
        match client.stations_at(resolved_time).await {
            Ok(stations) if has_enough_visible_stations(&stations) => {
                return Ok(LoadedPageData {
                    stations,
                    resolved_time,
                });
            }
            Ok(_) => {
                resolved_time -= DELTA_15MIN;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn has_enough_visible_stations(stations: &Stations) -> bool {
    !stations.is_empty() && visible_station_count(stations) >= 10
}

fn filter_stations(stations: &Stations, filter_query: &str) -> Vec<Station> {
    let stations = stations.as_ref();

    if filter_query.is_empty() {
        return stations.to_vec();
    }

    let haystacks = stations.iter().map(Station::nomestaz).collect::<Vec<_>>();
    let matches = match_list(filter_query, &haystacks, &Config::default());

    matches
        .into_iter()
        .filter_map(|matched| stations.get(matched.index as usize).cloned())
        .collect()
}

fn station_row(station: &Station) -> Row<'_> {
    let style = match station.value().unwrap_or(&f32::MIN) {
        x if *x > *station.soglia3() => tailwind::VIOLET.c500,
        x if *x > *station.soglia2() => tailwind::RED.c500,
        x if *x > *station.soglia1() => tailwind::YELLOW.c500,
        _ => tailwind::GREEN.c500,
    };

    Row::new([
        padded_cell(Line::from(station.nomestaz())),
        padded_cell(Line::from(
            station.value().copied().unwrap_or(0.0).to_string(),
        )),
        padded_cell(Line::from(station.soglia1().to_string())),
        padded_cell(Line::from(station.soglia2().to_string())),
        padded_cell(Line::from(station.soglia3().to_string())),
    ])
    .height(ITEM_HEIGHT as u16)
    .style(style)
}

fn padded_cell(content: Line<'_>) -> Cell<'_> {
    Cell::from(Text::from(vec![
        Line::default(),
        Line::from(content),
        Line::default(),
    ]))
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
    loaded_data: Option<&LoadedPageData>,
    time_input: &str,
    filter_query: &str,
    state: SelectionPageState,
) {
    let query_label = if matches!(state, SelectionPageState::Query) {
        format!("Time> {time_input}")
    } else {
        let time = loaded_data
            .map(|data| format_time(data.resolved_time))
            .unwrap_or_else(|| "loading...".to_owned());
        format!("Time: {time}")
    };

    let filter_label = if filter_query.is_empty() {
        "Filter: off".to_owned()
    } else if matches!(state, SelectionPageState::Filter) {
        format!("Filter> {filter_query}")
    } else {
        format!("Filter: {filter_query}")
    };

    let help = if matches!(state, SelectionPageState::Query) {
        QUERY_INFO_TEXT
    } else if matches!(state, SelectionPageState::Filter) {
        FILTER_INFO_TEXT
    } else {
        INFO_TEXT
    };

    let mut status_parts = vec![query_label, filter_label];
    if let Some(error) = error {
        status_parts.push(format!("Error: {error}"));
    }

    let status = status_parts.join(" | ");

    let footer = Paragraph::new(Text::from(vec![Line::from(status), Line::from(help)]))
        .block(Block::bordered().border_type(BorderType::Double));
    ratatui::widgets::Widget::render(footer, area, buf);
}

fn render_time_popup(buf: &mut Buffer, area: Rect, time_input: &str, error: Option<&str>) {
    let popup_area = centered_rect(area, 64, 7);
    let mut lines = vec![
        Line::from(vec![
            Span::raw("Time: "),
            Span::styled(time_input, Style::default().bg(Color::Black)),
        ]),
        Line::from("Format: YYYY-MM-DD HH:MM"),
    ];

    if let Some(error) = error {
        lines.push(Line::from(format!("Error: {error}")));
    }

    ratatui::widgets::Widget::render(Clear, popup_area, buf);
    ratatui::widgets::Widget::render(
        Paragraph::new(Text::from(lines)).block(
            Block::bordered()
                .border_type(BorderType::Double)
                .title(" Set time "),
        ),
        popup_area,
        buf,
    );
}

fn dim_area(buf: &mut Buffer, area: Rect) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                let style = cell.style().bg(Color::DarkGray);
                cell.set_style(style);
            }
        }
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height.min(area.height)),
        Constraint::Fill(1),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width.min(area.width)),
        Constraint::Fill(1),
    ])
    .split(vertical[1])[1]
}

#[inline(always)]
fn clamp_time_to_latest(time: DateTime<Local>) -> DateTime<Local> {
    std::cmp::min(
        clamp_station_time(time).unwrap(),
        latest_station_time().unwrap(),
    )
}

fn format_time(time: DateTime<Local>) -> String {
    time.format(QUERY_TIME_FORMAT).to_string()
}

fn parse_time_input(input: &str) -> Result<DateTime<Local>, String> {
    let naive = NaiveDateTime::parse_from_str(input.trim(), QUERY_TIME_FORMAT)
        .map_err(|_| format!("Invalid time. Use format {QUERY_TIME_FORMAT}"))?;

    match Local.from_local_datetime(&naive) {
        LocalResult::Single(time) => Ok(time),
        LocalResult::Ambiguous(_, _) => {
            Err("Ambiguous local time. Please choose a different minute.".to_owned())
        }
        LocalResult::None => {
            Err("Invalid local time. Please choose a different minute.".to_owned())
        }
    }
}
