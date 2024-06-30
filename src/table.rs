use std::ops::ControlFlow;

use crate::event_handler_trait::MutStatefulEventHandler;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, StatefulWidgetRef, Table, TableState, WidgetRef,
    },
};
use unicode_width::UnicodeWidthStr;

const INFO_TEXT: &str = "(Esc) quit | (↑) move up | (↓) move down | (󱞣) see graph";

const ITEM_HEIGHT: usize = 4;

#[derive(Clone)]
pub struct Data {
    name: String,
    address: String,
    email: String,
}

impl Data {
    const fn ref_array(&self) -> [&String; 3] {
        [&self.name, &self.address, &self.email]
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn address(&self) -> &str {
        &self.address
    }

    fn email(&self) -> &str {
        &self.email
    }
}
#[derive(Default)]
pub struct SelectionPageState {
    table_state: TableState,
    items: Vec<Data>,
    longest_item_lens: (u16, u16, u16), // order is (name, address, email)
    scroll_state: ScrollbarState,
}

impl SelectionPageState {
    pub fn new() -> Self {
        let data_vec = generate_fake_names();
        Self {
            table_state: TableState::default().with_selected(0),
            longest_item_lens: constraint_len_calculator(&data_vec),
            scroll_state: ScrollbarState::new((data_vec.len() - 1) * ITEM_HEIGHT),
            items: data_vec,
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

    pub fn get_selected_data(&mut self) -> Option<Data> {
        match self.table_state.selected() {
            Some(i) => Some(self.items[i].clone()),
            None => None,
        }
    }
}

fn generate_fake_names() -> Vec<Data> {
    use fakeit::{address, contact, name};

    (0..20)
        .map(|_| {
            let name = name::full();
            let address = format!(
                "{}\n{}, {} {}",
                address::street(),
                address::city(),
                address::state(),
                address::zip()
            );
            let email = contact::email();

            Data {
                name,
                address,
                email,
            }
        })
        .sorted_by(|a, b| a.name.cmp(&b.name))
        .collect_vec()
}
fn render_table(b: &mut Buffer, state: &mut SelectionPageState, area: Rect) {
    let header = ["Name", "Address", "Email"]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .height(1);
    let rows = state.items.iter().map(|data| {
        let item = data.ref_array();
        item.into_iter()
            .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
            .collect::<Row>()
            .height(4)
    });
    let bar = " █ ";
    let t = Table::new(
        rows,
        [
            // + 1 is for padding.
            Constraint::Length(state.longest_item_lens.0 + 1),
            Constraint::Min(state.longest_item_lens.1 + 1),
            Constraint::Min(state.longest_item_lens.2),
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

fn constraint_len_calculator(items: &[Data]) -> (u16, u16, u16) {
    let name_len = items
        .iter()
        .map(Data::name)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let address_len = items
        .iter()
        .map(Data::address)
        .flat_map(str::lines)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let email_len = items
        .iter()
        .map(Data::email)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (name_len as u16, address_len as u16, email_len as u16)
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

impl MutStatefulEventHandler<SelectionPageState, (), Option<Data>> for SelectionPage {
    fn handle(
        &mut self,
        event: Event,
        state: Option<&mut SelectionPageState>,
    ) -> ControlFlow<(), Option<Data>> {
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
