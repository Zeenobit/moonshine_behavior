use std::fmt::Debug;

use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use moonshine_kind::Instance;

use crate::events::TransitionEvent;
use crate::{sequence::TransitionSequence, Behavior, BehaviorMut, Memory, TransitionEventsMut};

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

pub fn transition<T: Behavior + Component>(
    mut events: TransitionEventsMut<T>,
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
        let mut stop_index = None;
        match behavior.transition.take() {
            Next(next) => {
                behavior.push(instance, next, &mut events);
            }
            Interrupt(next) => {
                behavior.interrupt(instance, next, &mut events);
                continue;
            }
            Previous => {
                stop_index = Some(behavior.index());
                behavior.pop(instance, &mut events);
            }
            Reset => behavior.clear(instance, &mut events),
            _ => {
                if behavior.current.is_added() {
                    // Send start event for the initial behavior
                    debug_assert!(behavior.memory.is_empty());
                    events.send(TransitionEvent::Start { instance, index: 0 });
                }
            }
        }

        if let Some(sequence) = sequence_opt {
            if sequence.is_empty() {
                commands.entity(*instance).remove::<TransitionSequence<T>>();
            } else {
                TransitionSequence::update(sequence, behavior, stop_index);
            }
        }
    }
}

pub type TransitionChanged<T> = Or<(Changed<Transition<T>>, With<TransitionSequence<T>>)>;

#[derive(Debug, PartialEq, Reflect)]
pub enum TransitionError<T: Behavior> {
    RejectedNext(T),
    NoPrevious,
}
