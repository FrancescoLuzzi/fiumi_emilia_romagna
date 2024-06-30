use std::ops::ControlFlow;

use crossterm::event::Event;

pub trait MutStatefulEventHandler<S, B, C> {
    fn handle(&mut self, event: Event, state: Option<&mut S>) -> ControlFlow<B, C>;
}
pub trait StatefulEventHandler<S, B, C> {
    fn handle(&self, event: Event, state: Option<&S>) -> ControlFlow<B, C>;
}

pub trait MutEventHandler<B, C> {
    fn handle(&mut self, event: Event) -> ControlFlow<B, C>;
}
pub trait EventHandler<B, C> {
    fn handle(&self, event: Event) -> ControlFlow<B, C>;
}
