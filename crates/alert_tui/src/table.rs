use crate::app::UiAction;
use alert_core::model::Station;
use crossterm::event::{Event, KeyCode, KeyEventKind};
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
use unicode_width::UnicodeWidthStr;

const INFO_TEXT: &str = "(Esc) quit | (↑) move up | (↓) move down | (Enter) see graph";
const ITEM_HEIGHT: usize = 4;

pub struct SelectionPage {
    table_state: TableState,
    items: Vec<Station>,
    longest_item_lens: (u16, u16, u16, u16, u16),
    scroll_state: ScrollbarState,
    error: Option<String>,
}

impl SelectionPage {
    pub fn new(items: Vec<Station>) -> Self {
        Self {
            table_state: TableState::default().with_selected(0),
            longest_item_lens: constraint_len_calculator(&items),
            scroll_state: ScrollbarState::new((items.len().saturating_sub(1)) * ITEM_HEIGHT),
            items,
            error: None,
        }
    }

    pub fn set_items(&mut self, items: Vec<Station>) {
        self.longest_item_lens = constraint_len_calculator(&items);
        self.scroll_state =
            ScrollbarState::new((items.len().saturating_sub(1)).saturating_mul(ITEM_HEIGHT));
        self.items = items;
        let selected = self
            .table_state
            .selected()
            .filter(|idx| *idx < self.items.len())
            .unwrap_or(0);
        self.table_state
            .select((!self.items.is_empty()).then_some(selected));
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }

    pub fn handle_event(&mut self, event: Event) -> UiAction {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                return match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => UiAction::Quit,
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.next();
                        UiAction::Redraw
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.previous();
                        UiAction::Redraw
                    }
                    KeyCode::Enter => self
                        .selected_station()
                        .map(|station| UiAction::OpenGraph { station })
                        .unwrap_or(UiAction::Redraw),
                    _ => UiAction::Redraw,
                };
            }
        }

        UiAction::Redraw
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);
        self.render_table(buf, rects[0]);
        self.render_scrollbar(buf, rects[0]);
        render_footer(buf, rects[1], self.error.as_deref());
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

fn render_footer(buf: &mut Buffer, area: Rect, error: Option<&str>) {
    let text = error.unwrap_or(INFO_TEXT);
    let footer = Paragraph::new(Line::from(text))
        .centered()
        .block(Block::bordered().border_type(BorderType::Double));
    ratatui::widgets::Widget::render(footer, area, buf);
}
