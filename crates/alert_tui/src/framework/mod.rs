use crossterm::event::Event;
use ratatui::{buffer::Buffer, layout::Rect};
use std::{collections::HashMap, future::Future, hash::Hash, pin::Pin};

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
pub type TaskKey = &'static str;

pub enum AppMessage<E> {
    Input(Event),
    AppEvent(E),
    Shutdown,
}

pub enum Task<M> {
    None,
    Future {
        key: Option<TaskKey>,
        future: BoxFuture<M>,
    },
    Batch(Vec<Task<M>>),
}

impl<M> Default for Task<M> {
    fn default() -> Self {
        Self::None
    }
}

impl<M: 'static> Task<M> {
    pub fn none() -> Self {
        Self::None
    }

    pub fn future<F>(future: F) -> Self
    where
        F: Future<Output = M> + Send + 'static,
    {
        Self::Future {
            key: None,
            future: Box::pin(future),
        }
    }

    pub fn keyed<F>(key: TaskKey, future: F) -> Self
    where
        F: Future<Output = M> + Send + 'static,
    {
        Self::Future {
            key: Some(key),
            future: Box::pin(future),
        }
    }

    pub fn perform<T: Send + 'static, Fut, Map>(future: Fut, map: Map) -> Self
    where
        Fut: Future<Output = T> + Send + 'static,
        Map: FnOnce(T) -> M + Send + 'static,
    {
        Self::future(async move { map(future.await) })
    }

    pub fn batch<I>(tasks: I) -> Self
    where
        I: IntoIterator<Item = Task<M>>,
    {
        let tasks = tasks
            .into_iter()
            .filter(|task| !matches!(task, Task::None))
            .collect::<Vec<_>>();

        match tasks.len() {
            0 => Self::None,
            1 => tasks.into_iter().next().expect("single task should exist"),
            _ => Self::Batch(tasks),
        }
    }

    pub fn map<N: 'static, F>(self, f: F) -> Task<N>
    where
        F: Fn(M) -> N + Clone + Send + Sync + 'static,
    {
        match self {
            Self::None => Task::None,
            Self::Future { key, future } => Task::Future {
                key,
                future: Box::pin(async move { f(future.await) }),
            },
            Self::Batch(tasks) => Task::batch(tasks.into_iter().map(|task| task.map(f.clone()))),
        }
    }
}

pub struct Update<A, M> {
    pub redraw: bool,
    pub action: Option<A>,
    pub task: Task<M>,
}

impl<A, M: 'static> Update<A, M> {
    pub fn none() -> Self {
        Self {
            redraw: false,
            action: None,
            task: Task::none(),
        }
    }

    pub fn redraw() -> Self {
        Self {
            redraw: true,
            action: None,
            task: Task::none(),
        }
    }

    pub fn action(action: A) -> Self {
        Self {
            redraw: false,
            action: Some(action),
            task: Task::none(),
        }
    }

    pub fn task(task: Task<M>) -> Self {
        Self {
            redraw: false,
            action: None,
            task,
        }
    }

    pub fn and_redraw(mut self) -> Self {
        self.redraw = true;
        self
    }

    pub fn with_task(mut self, task: Task<M>) -> Self {
        self.task = Task::batch([self.task, task]);
        self
    }

    pub fn with_tasks<I>(mut self, tasks: I) -> Self
    where
        I: IntoIterator<Item = Task<M>>,
    {
        self.task = Task::batch(std::iter::once(self.task).chain(tasks));
        self
    }

    pub fn map_action<B, F>(self, f: F) -> Update<B, M>
    where
        F: FnOnce(A) -> B,
    {
        Update {
            redraw: self.redraw,
            action: self.action.map(f),
            task: self.task,
        }
    }

    pub fn map_message<N: 'static, F>(self, f: F) -> Update<A, N>
    where
        F: Fn(M) -> N + Clone + Send + Sync + 'static,
    {
        Update {
            redraw: self.redraw,
            action: self.action,
            task: self.task.map(f),
        }
    }
}

pub trait PageModel {
    type Action;
    type Message: Send + 'static;

    fn init(&mut self) -> Update<Self::Action, Self::Message> {
        Update::none()
    }

    fn handle_event(&mut self, event: Event) -> Update<Self::Action, Self::Message>;

    fn update(&mut self, message: Self::Message) -> Update<Self::Action, Self::Message>;
}

pub trait RenderablePageModel: PageModel {
    fn render(&mut self, area: Rect, buf: &mut Buffer);

    fn cursor_position(&self, _area: Rect) -> Option<(u16, u16)> {
        None
    }
}

pub struct MultiPageFrame<T, P>
where
    T: Copy + Eq + Hash,
    P: RenderablePageModel,
{
    pages: HashMap<T, P>,
    active_page: T,
}

impl<T, P> MultiPageFrame<T, P>
where
    T: Copy + Eq + Hash,
    P: RenderablePageModel,
{
    pub fn new(pages: HashMap<T, P>, active_page: T) -> Self {
        Self { pages, active_page }
    }

    pub fn insert(&mut self, page_id: T, page: P) {
        self.pages.insert(page_id, page);
    }

    pub fn insert_and_show(&mut self, page_id: T, page: P) {
        self.insert(page_id, page);
        self.active_page = page_id;
    }

    pub fn contains_page(&self, page_id: T) -> bool {
        self.pages.contains_key(&page_id)
    }

    pub fn remove_page(&mut self, page_id: T) -> Option<P> {
        self.pages.remove(&page_id)
    }

    pub fn set_active_page(&mut self, page_id: T) {
        self.active_page = page_id;
    }

    pub fn show(&mut self, page_id: T) -> bool {
        if self.contains_page(page_id) {
            self.active_page = page_id;
            true
        } else {
            false
        }
    }

    pub fn active_page(&self) -> T {
        self.active_page
    }

    pub fn update_at(
        &mut self,
        page_id: T,
        message: P::Message,
    ) -> Option<Update<P::Action, P::Message>> {
        self.pages.get_mut(&page_id).map(|page| page.update(message))
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.current_page_mut().render(area, buf);
    }

    pub fn cursor_position(&self, area: Rect) -> Option<(u16, u16)> {
        self.pages.get(&self.active_page)?.cursor_position(area)
    }

    fn current_page_mut(&mut self) -> &mut P {
        self.pages
            .get_mut(&self.active_page)
            .expect("active page should be registered")
    }
}

impl<T, P> PageModel for MultiPageFrame<T, P>
where
    T: Copy + Eq + Hash,
    P: RenderablePageModel,
{
    type Action = P::Action;
    type Message = P::Message;

    fn init(&mut self) -> Update<Self::Action, Self::Message> {
        self.current_page_mut().init()
    }

    fn handle_event(&mut self, event: Event) -> Update<Self::Action, Self::Message> {
        self.current_page_mut().handle_event(event)
    }

    fn update(&mut self, message: Self::Message) -> Update<Self::Action, Self::Message> {
        self.current_page_mut().update(message)
    }
}
