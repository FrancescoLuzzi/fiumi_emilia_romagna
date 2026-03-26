use super::config::UiConfig;
use anyhow::Context;
use async_channel::{Receiver, Sender};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::Backend;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::task::JoinHandle;

use super::{AppMessage, Task, TaskKey};

pub struct AppReaction<T> {
    pub redraw: bool,
    pub should_quit: bool,
    pub task: Task<AppMessage<T>>,
}

impl<T> Default for AppReaction<T> {
    fn default() -> Self {
        Self {
            redraw: false,
            should_quit: false,
            task: Task::None,
        }
    }
}

pub trait AppModel {
    type Event;

    fn init(&mut self) -> AppReaction<Self::Event> {
        AppReaction::default()
    }

    fn handle_message(&mut self, message: AppMessage<Self::Event>) -> AppReaction<Self::Event>;

    fn render(&mut self, frame: &mut Frame);
}

pub async fn spawn_input_task<T: Send + Sync + 'static>(sender: Sender<AppMessage<T>>) {
    std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(event) => {
                    if sender.send_blocking(AppMessage::<T>::Input(event)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = sender.send_blocking(AppMessage::<T>::Shutdown);
                    break;
                }
            }
        }
    });
}

async fn execute_task<T>(
    sender: &Sender<AppMessage<T>>,
    task: Task<AppMessage<T>>,
    pending_tasks: &mut HashMap<TaskKey, JoinHandle<()>>,
) where
    T: Send + Sync + 'static,
{
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

async fn draw_if_due<B: Backend, A: AppModel>(
    terminal: &mut Terminal<B>,
    app: &mut A,
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

fn should_stop_event_batch(last_draw_at: Instant, config: UiConfig) -> bool {
    const FRAME_DEADLINE_GUARD: Duration = Duration::from_millis(1);

    let next_frame_at = last_draw_at + config.frame_interval();
    next_frame_at.saturating_duration_since(Instant::now()) <= FRAME_DEADLINE_GUARD
}

pub async fn run_app<B: Backend, A>(
    terminal: &mut Terminal<B>,
    mut app: A,
    config: UiConfig,
    receiver: Receiver<AppMessage<A::Event>>,
    sender: Sender<AppMessage<A::Event>>,
) -> anyhow::Result<()>
where
    A: AppModel,
    A::Event: Send + Sync + 'static,
{
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
            if should_stop_event_batch(last_draw_at, config) {
                break;
            }

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
