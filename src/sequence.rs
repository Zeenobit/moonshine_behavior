use std::collections::VecDeque;

use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

use crate::Behavior;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Sequence<T: Behavior> {
    queue: VecDeque<T>,
}

impl<T: Behavior> Sequence<T> {
    pub fn new(items: impl IntoIterator<Item = T>) -> Self {
        Self {
            queue: VecDeque::from_iter(items),
        }
    }

    pub fn empty() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn start(next: T) -> Self {
        let mut sequence = Self::empty();
        sequence.push(next);
        sequence
    }

    pub fn then(mut self, next: T) -> Self {
        self.push(next);
        self
    }

    pub fn push(&mut self, next: T) {
        self.queue.push_back(next);
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        self.queue.pop_front()
    }
}
