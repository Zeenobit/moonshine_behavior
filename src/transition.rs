use std::collections::VecDeque;
use std::fmt::Debug;

use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_utils::tracing::debug;
use moonshine_kind::prelude::*;

use crate::events::BehaviorEvent;
use crate::{Behavior, BehaviorEventsMut, BehaviorExt, BehaviorMut, BehaviorMutItem, Memory};

pub use self::Transition::{Interrupt, Next, Previous, Reset};

#[derive(Component, Debug, Reflect)]
#[require(Memory<T>)]
#[reflect(Component)]
pub enum Transition<T: Behavior> {
    None,
    Next(T),
    Interrupt(T),
    Previous,
    Reset,
}

impl<T: Behavior> Transition<T> {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    fn take(&mut self) -> Self {
        std::mem::replace(self, Transition::None)
    }
}

impl<T: Behavior> Default for Transition<T> {
    fn default() -> Self {
        Self::None
    }
}

impl<T: Behavior + Clone> Clone for Transition<T> {
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Next(next) => Next(next.clone()),
            Interrupt(next) => Interrupt(next.clone()),
            Previous => Previous,
            Reset => Reset,
        }
    }
}

pub fn transition<T: Behavior>(
    mut events: BehaviorEventsMut<T>,
    mut query: Query<
        (
            Instance<T>,
            BehaviorMut<T>,
            Option<&mut TransitionSequence<T>>,
        ),
        TransitionChanged<T>,
    >,
    mut commands: Commands,
) {
    for (instance, mut behavior, sequence_opt) in &mut query {
        if behavior.current.is_added() {
            // Memory must be empty when the component is added
            debug_assert!(behavior.memory.is_empty());

            // Send start event for the initial behavior
            behavior.invoke_start(None, commands.instance(instance));
            events.send(BehaviorEvent::Start { instance, index: 0 });
        }

        // Index of the stopped behavior, if applicable.
        let mut stop_index = None;

        let mut interrupt_sequence = false;

        match behavior.transition.take() {
            Next(next) => {
                interrupt_sequence = !behavior.push(instance, next, &mut events, &mut commands);
            }
            Previous => {
                stop_index = Some(behavior.index());
                interrupt_sequence = !behavior.pop(instance, &mut events, &mut commands);
            }
            Interrupt(next) => {
                behavior.interrupt(instance, next, &mut events, &mut commands);
                interrupt_sequence = true;
            }
            Reset => {
                behavior.clear(instance, &mut events, &mut commands);
                interrupt_sequence = true;
            }
            _ => {}
        }

        let Some(sequence) = sequence_opt else {
            continue;
        };

        if interrupt_sequence {
            debug!("{instance:?}: sequence interrupted");
            commands.entity(*instance).remove::<TransitionSequence<T>>();
        } else if sequence.is_empty() {
            debug!("{instance:?}: sequence finished");
            commands.entity(*instance).remove::<TransitionSequence<T>>();
        } else {
            TransitionSequence::update(sequence, instance, behavior, stop_index);
        }
    }
}

// TODO: Can we use `Changed<TransitionSequence<T>>` for this?
pub type TransitionChanged<T> = Or<(Changed<Transition<T>>, With<TransitionSequence<T>>)>;

#[derive(Debug, PartialEq, Reflect)]
pub enum TransitionError<T: Behavior> {
    RejectedNext(T),
    NoPrevious,
}

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

    pub fn wait_for(next: T) -> Self {
        Self::empty().then_wait_for(next)
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

impl<T: Behavior> TransitionSequence<T> {
    pub(crate) fn update(
        mut this: Mut<Self>,
        instance: Instance<T>,
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

        debug!("{instance:?}: sequence steps = {:?}", this.queue.len());

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
