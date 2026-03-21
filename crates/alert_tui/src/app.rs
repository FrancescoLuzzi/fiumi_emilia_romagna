use crate::{
    config::UiConfig,
    framework::{AppMessage, MultiPageFrame, PageModel, RenderablePageModel, Task, TaskKey, Update},
    pages::{graph, graph::GraphPage, selection, selection::SelectionPage},
};
use anyhow::Context;
use async_channel::{Receiver, Sender};
use crossterm::event::Event;
use ratatui::{Frame, Terminal, backend::Backend, buffer::Buffer, layout::Rect};
use std::{collections::HashMap, time::Instant};
use tokio::task::JoinHandle;

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

    pub fn render(&mut self, frame: &mut Frame) {
        self.pages.render(frame.area(), frame.buffer_mut());
    }

    pub fn active_page(&self) -> PageId {
        self.pages.active_page()
    }

    fn show_selection(&mut self) {
        let _ = self.pages.show(PageId::Selection);
    }

    fn show_graph(
        &mut self,
        station: alert_core::model::Station,
    ) -> Update<PageAction, Message> {
        self.pages
            .insert_and_show(PageId::Graph, Page::Graph(GraphPage::loading(station)));
        self.pages.init()
    }

    fn close_graph(&mut self) {
        let _ = self.pages.remove_page(PageId::Graph);
    }

    fn handle_page_update(
        &mut self,
        update: Update<PageAction, Message>,
    ) -> AppReaction {
        let mut reaction = AppReaction {
            redraw: update.redraw,
            task: update.task,
            ..AppReaction::default()
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

    pub fn init(&mut self) -> AppReaction {
        let update = self.pages.init();
        self.handle_page_update(update)
    }

    pub fn handle_message(&mut self, message: Message) -> AppReaction {
        match message {
            Message::Input(event) => {
                let update = self.pages.handle_event(event);
                self.handle_page_update(update)
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
                ..AppReaction::default()
            },
        }
    }
}

#[derive(Default)]
pub struct AppReaction {
    pub redraw: bool,
    pub should_quit: bool,
    pub task: Task<Message>,
}

pub async fn spawn_input_task(sender: Sender<Message>) {
    std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(event) => {
                    if sender.send_blocking(Message::Input(event)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = sender.send_blocking(Message::Shutdown);
                    break;
                }
            }
        }
    });
}

async fn execute_task(
    sender: &Sender<Message>,
    task: Task<Message>,
    pending_tasks: &mut HashMap<TaskKey, JoinHandle<()>>,
) {
    let mut queue = vec![task];

    while let Some(task) = queue.pop() {
        match task {
            Task::None => {}
            Task::Future { key, future } => {
                if let Some(task_key) = key {
                    if let Some(handle) = pending_tasks.remove(&task_key) {
                        handle.abort();
                    }

                    let sender = sender.clone();
                    let handle = tokio::spawn(async move {
                        let message = future.await;
                        let _ = sender.send(message).await;
                    });
                    pending_tasks.insert(task_key, handle);
                } else {
                    let sender = sender.clone();
                    tokio::spawn(async move {
                        let message = future.await;
                        let _ = sender.send(message).await;
                    });
                }
            }
            Task::Batch(tasks) => {
                queue.extend(tasks.into_iter().rev());
            }
        }
    }
}

async fn draw_if_due<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
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

pub async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    config: UiConfig,
    receiver: Receiver<Message>,
    sender: Sender<Message>,
) -> anyhow::Result<()> {
    let mut dirty = true;
    let mut last_draw_at = Instant::now() - config.frame_interval();
    let mut pending_tasks = HashMap::new();

    let mut reaction = app.init();
    dirty |= reaction.redraw;
    execute_task(&sender, reaction.task, &mut pending_tasks).await;

    draw_if_due(terminal, &mut app, &mut dirty, &mut last_draw_at, config).await?;

    loop {
        let first_message = receiver.recv().await.context("event channel closed")?;
        reaction = app.handle_message(first_message);
        dirty |= reaction.redraw;
        if reaction.should_quit {
            return Ok(());
        }
        execute_task(&sender, reaction.task, &mut pending_tasks).await;

        for _ in 1..config.max_events_per_batch {
            let Ok(message) = receiver.try_recv() else {
                break;
            };
            reaction = app.handle_message(message);
            dirty |= reaction.redraw;
            execute_task(&sender, reaction.task, &mut pending_tasks).await;
            if reaction.should_quit {
                return Ok(());
            }
        }

        draw_if_due(terminal, &mut app, &mut dirty, &mut last_draw_at, config).await?;
    }
}

pub async fn bootstrap(config: UiConfig) -> (App, Sender<Message>, Receiver<Message>) {
    let (sender, receiver) = async_channel::bounded::<Message>(256);

    spawn_input_task(sender.clone()).await;

    let mut pages = HashMap::new();
    pages.insert(
        PageId::Selection,
        Page::Selection(SelectionPage::new(
            Vec::new(),
            config.filter_debounce_interval(),
        )),
    );

    let frame = MultiPageFrame::new(pages, PageId::Selection);

    (App::new(frame), sender, receiver)
}
