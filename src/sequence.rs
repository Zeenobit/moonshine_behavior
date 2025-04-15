use std::collections::VecDeque;

use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

use crate::{Behavior, BehaviorMutItem};

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TransitionSequence<T: Behavior> {
    queue: VecDeque<TransitionSequenceElement<T>>,
    wait_index: Option<usize>,
}

impl<T: Behavior> TransitionSequence<T> {
    pub fn new(items: impl IntoIterator<Item = T>) -> Self {
        Self {
            queue: VecDeque::from_iter(items.into_iter().map(TransitionSequenceElement::Start)),
            wait_index: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            queue: VecDeque::new(),
            wait_index: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn start(next: T) -> Self {
        let mut sequence = Self::empty();
        sequence.push(TransitionSequenceElement::Start(next));
        sequence
    }

    pub fn then(mut self, next: T) -> Self {
        self.push(TransitionSequenceElement::Start(next));
        self
    }

    pub fn then_wait_for(mut self, next: T) -> Self {
        self.push(TransitionSequenceElement::StartWait(next));
        self
    }

    pub fn then_stop(mut self) -> Self {
        self.push(TransitionSequenceElement::Stop);
        self
    }

    fn push(&mut self, next: TransitionSequenceElement<T>) {
        self.queue.push_back(next);
    }
}

impl<T: Behavior + Component> TransitionSequence<T> {
    pub(crate) fn update(
        mut this: Mut<Self>,
        mut behavior: BehaviorMutItem<T>,
        stop_index: Option<usize>,
    ) {
        debug_assert!(!this.queue.is_empty());

        if let Some(wait_index) = this.wait_index {
            if let Some(stop_index) = stop_index {
                if wait_index != stop_index {
                    return;
                }
            } else {
                return;
            }
        }

        if let Some(element) = this.queue.pop_front() {
            use TransitionSequenceElement::*;
            match element {
                Start(next) => {
                    this.wait_index = None;
                    behavior.start(next);
                }
                StartWait(next) => {
                    this.wait_index = Some(behavior.index() + 1);
                    behavior.start(next);
                }
                Stop => {
                    this.wait_index = None;
                    behavior.stop();
                }
            }
        }
    }
}

#[derive(Debug, Reflect)]
enum TransitionSequenceElement<T: Behavior> {
    Start(T),
    StartWait(T),
    Stop,
}
