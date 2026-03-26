use crate::{
    framework::{
        AppMessage, AppModel, AppReaction, MultiPageFrame, PageModel, RenderablePageModel,
        Task, UiConfig, Update, spawn_input_task,
    },
    pages::{graph, graph::GraphPage, selection, selection::SelectionPage},
};
use async_channel::{Receiver, Sender};
use crossterm::event::Event;
use ratatui::{Frame, buffer::Buffer, layout::Rect};
use std::collections::HashMap;

pub enum AppEvent {
    Selection(selection::Message),
    Graph(graph::Message),
}

type Message = AppMessage<AppEvent>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PageId {
    Selection,
    Graph,
}

pub enum Page {
    Selection(SelectionPage),
    Graph(GraphPage),
}

impl PageModel for Page {
    type Action = PageAction;
    type Message = Message;

    fn init(&mut self) -> Update<Self::Action, Self::Message> {
        match self {
            Page::Selection(page) => page
                .init()
                .map_action(PageAction::Selection)
                .map_message(|message| Message::AppEvent(AppEvent::Selection(message))),
            Page::Graph(page) => page
                .init()
                .map_action(PageAction::Graph)
                .map_message(|message| Message::AppEvent(AppEvent::Graph(message))),
        }
    }

    fn handle_event(&mut self, event: Event) -> Update<Self::Action, Self::Message> {
        match self {
            Page::Selection(page) => page
                .handle_event(event)
                .map_action(PageAction::Selection)
                .map_message(|message| Message::AppEvent(AppEvent::Selection(message))),
            Page::Graph(page) => page
                .handle_event(event)
                .map_action(PageAction::Graph)
                .map_message(|message| Message::AppEvent(AppEvent::Graph(message))),
        }
    }

    fn update(&mut self, message: Self::Message) -> Update<Self::Action, Self::Message> {
        match (self, message) {
            (Page::Selection(page), Message::AppEvent(AppEvent::Selection(message))) => page
                .update(message)
                .map_action(PageAction::Selection)
                .map_message(|message| Message::AppEvent(AppEvent::Selection(message))),
            (Page::Graph(page), Message::AppEvent(AppEvent::Graph(message))) => page
                .update(message)
                .map_action(PageAction::Graph)
                .map_message(|message| Message::AppEvent(AppEvent::Graph(message))),
            (_, _) => Update::none(),
        }
    }
}

impl RenderablePageModel for Page {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        match self {
            Page::Selection(page) => page.render(area, buf),
            Page::Graph(page) => page.render(area, buf),
        }
    }

    fn cursor_position(&self, area: Rect) -> Option<(u16, u16)> {
        match self {
            Page::Selection(page) => page.cursor_position(area),
            Page::Graph(page) => page.cursor_position(area),
        }
    }
}

pub enum PageAction {
    Selection(selection::Action),
    Graph(graph::Action),
}

pub struct App {
    pages: MultiPageFrame<PageId, Page>,
}

impl App {
    pub fn new(pages: MultiPageFrame<PageId, Page>) -> Self {
        Self { pages }
    }

    pub fn active_page(&self) -> PageId {
        self.pages.active_page()
    }

    fn show_selection(&mut self) {
        let _ = self.pages.show(PageId::Selection);
    }

    fn show_graph(&mut self, station: alert_core::model::Station) -> Update<PageAction, Message> {
        self.pages
            .insert_and_show(PageId::Graph, Page::Graph(GraphPage::loading(station)));
        self.pages.init()
    }

    fn close_graph(&mut self) {
        let _ = self.pages.remove_page(PageId::Graph);
    }

    fn handle_page_update(&mut self, update: Update<PageAction, Message>) -> AppReaction<AppEvent> {
        let mut reaction = AppReaction {
            redraw: update.redraw,
            task: update.task,
            should_quit: false,
        };

        match update.action {
            None => reaction,
            Some(PageAction::Selection(selection::Action::Quit)) => {
                reaction.should_quit = true;
                reaction
            }
            Some(PageAction::Selection(selection::Action::OpenGraph { station })) => {
                let init = self.show_graph(station);
                reaction.redraw = true;
                reaction.redraw |= init.redraw;
                reaction.task = Task::batch([reaction.task, init.task]);
                reaction
            }
            Some(PageAction::Graph(graph::Action::Back)) => {
                self.close_graph();
                self.show_selection();
                reaction.redraw = true;
                reaction
            }
        }
    }
}

impl AppModel for App {
    type Event = AppEvent;

    fn init(&mut self) -> AppReaction<Self::Event> {
        let update = self.pages.init();
        self.handle_page_update(update)
    }

    fn handle_message(&mut self, message: AppMessage<Self::Event>) -> AppReaction<Self::Event> {
        match message {
            Message::Input(event) => {
                let is_resize = matches!(event, Event::Resize(_, _));
                let update = self.pages.handle_event(event);
                let mut reaction = self.handle_page_update(update);
                reaction.redraw |= is_resize;
                reaction
            }
            Message::AppEvent(AppEvent::Selection(message)) => self
                .pages
                .update_at(
                    PageId::Selection,
                    Message::AppEvent(AppEvent::Selection(message)),
                )
                .map(|update| self.handle_page_update(update))
                .unwrap_or_default(),
            Message::AppEvent(AppEvent::Graph(message)) => self
                .pages
                .update_at(PageId::Graph, Message::AppEvent(AppEvent::Graph(message)))
                .map(|update| self.handle_page_update(update))
                .unwrap_or_default(),
            Message::Shutdown => AppReaction {
                should_quit: true,
                ..Default::default()
            },
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        self.pages.render(frame.area(), frame.buffer_mut());

        if let Some((x, y)) = self.pages.cursor_position(frame.area()) {
            frame.set_cursor_position((x, y));
        }
    }
}

pub async fn bootstrap(config: UiConfig) -> (App, Sender<Message>, Receiver<Message>) {
    let (sender, receiver) = async_channel::bounded::<Message>(256);

    spawn_input_task(sender.clone()).await;

    let mut pages = HashMap::new();
    pages.insert(
        PageId::Selection,
        Page::Selection(SelectionPage::new(config.filter_debounce_interval())),
    );

    let frame = MultiPageFrame::new(pages, PageId::Selection);

    (App::new(frame), sender, receiver)
}
