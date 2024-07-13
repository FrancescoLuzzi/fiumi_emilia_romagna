use std::ops::ControlFlow;

use crate::event_handler_trait::MutStatefulEventHandler;
use crate::Station;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    style::{palette::tailwind, Color},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, StatefulWidgetRef, Table, TableState, WidgetRef,
    },
};
use unicode_width::UnicodeWidthStr;

const INFO_TEXT: &str = "(Esc) quit | (↑) move up | (↓) move down | (󱞣) see graph";

const ITEM_HEIGHT: usize = 4;

#[derive(Default)]
pub struct SelectionPageState {
    table_state: TableState,
    items: Vec<Station>,
    longest_item_lens: (u16, u16, u16, u16, u16), // order is (name, address, email)
    scroll_state: ScrollbarState,
}

impl SelectionPageState {
    pub fn new(stations: Vec<Station>) -> Self {
        Self {
            table_state: TableState::default().with_selected(0),
            longest_item_lens: constraint_len_calculator(&stations),
            scroll_state: ScrollbarState::new((stations.len().saturating_sub(1)) * ITEM_HEIGHT),
            items: stations,
        }
    }
    pub fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn get_selected_data(&mut self) -> Option<Station> {
        match self.table_state.selected() {
            Some(i) => Some(self.items[i].clone()),
            None => None,
        }
    }
}

fn render_table(b: &mut Buffer, state: &mut SelectionPageState, area: Rect) {
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
    let rows = state.items.iter().map(|data| {
        let item = data.ref_array();
        let style = match data.value().unwrap_or(&f32::MIN) {
            _x if _x > data.soglia3() => tailwind::VIOLET.c500,
            _x if _x > data.soglia2() => tailwind::RED.c500,
            _x if _x > data.soglia1() => tailwind::YELLOW.c500,
            _ => tailwind::GREEN.c500,
        };
        item.into_iter()
            .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
            .collect::<Row>()
            .height(ITEM_HEIGHT as u16)
            .style(style)
    });
    let bar = " █ ";
    let t = Table::new(
        rows,
        [
            // + 1 is for padding.
            Constraint::Length(state.longest_item_lens.0 + 1),
            Constraint::Min(state.longest_item_lens.1 + 1),
            Constraint::Min(state.longest_item_lens.2 + 1),
            Constraint::Min(state.longest_item_lens.3 + 1),
            Constraint::Min(state.longest_item_lens.4),
        ],
    )
    .header(header)
    .highlight_symbol(Text::from(vec![
        "".into(),
        bar.into(),
        bar.into(),
        "".into(),
    ]))
    .highlight_spacing(HighlightSpacing::Always);

    StatefulWidgetRef::render_ref(&t, area, b, &mut state.table_state);
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

    #[allow(clippy::cast_possible_truncation)]
    (
        nomestaz_len as u16,
        value_len as u16,
        soglia1_len as u16,
        soglia2_len as u16,
        soglia3_len as u16,
    )
}

fn render_scrollbar(b: &mut Buffer, state: &mut SelectionPageState, area: Rect) {
    Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .render(
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            b,
            &mut state.scroll_state,
        )
}

fn render_footer(b: &mut Buffer, area: Rect) {
    let info_footer = Paragraph::new(Line::from(INFO_TEXT))
        .centered()
        .block(Block::bordered().border_type(BorderType::Double));
    info_footer.render_ref(area, b);
}

pub struct SelectionPage {}
impl StatefulWidgetRef for SelectionPage {
    type State = SelectionPageState;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);

        render_table(buf, state, rects[0]);

        render_scrollbar(buf, state, rects[0]);

        render_footer(buf, rects[1]);
    }
}

impl MutStatefulEventHandler<SelectionPageState, (), Option<Station>> for SelectionPage {
    fn handle(
        &mut self,
        event: Event,
        state: Option<&mut SelectionPageState>,
    ) -> ControlFlow<(), Option<Station>> {
        let state = state.unwrap();
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                return match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => ControlFlow::Break(()),
                    KeyCode::Char('j') | KeyCode::Down => {
                        state.next();
                        ControlFlow::Continue(None)
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.previous();
                        ControlFlow::Continue(None)
                    }
                    KeyCode::Enter => ControlFlow::Continue(state.get_selected_data()),
                    _ => ControlFlow::Continue(None),
                };
            }
        }
        ControlFlow::Continue(None)
    }
}
