use std::fmt::Debug;

use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use moonshine_kind::Instance;

use crate::{sequence::Sequence, Behavior, BehaviorEventsMut, BehaviorMut, Memory};

pub use self::Transition::{Next, Previous, Reset};

#[derive(Component, Debug, Reflect)]
#[require(Memory<T>)]
#[reflect(Component)]
pub enum Transition<T: Behavior> {
    None,
    Next(T),
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
            Previous => Previous,
            Reset => Reset,
        }
    }
}

pub fn transition<T: Behavior + Component>(
    mut events: BehaviorEventsMut<T>,
    mut query: Query<(Instance<T>, BehaviorMut<T>, Option<&mut Sequence<T>>), TransitionChanged<T>>,
) {
    for (instance, mut behavior, sequence_opt) in &mut query {
        match behavior.transition.take() {
            Next(next) => {
                behavior.push(instance, next, &mut events);
                continue;
            }
            Previous => behavior.pop(instance, &mut events),
            Reset => behavior.clear(instance, &mut events),
            _ => {
                if behavior.current.is_added() {
                    events.start(instance);
                }
            }
        }

        if let Some(next) = sequence_opt.map(|mut sequence| sequence.pop()).flatten() {
            behavior.push(instance, next, &mut events)
        }
    }
}

pub type TransitionChanged<T> = Or<(Changed<Transition<T>>, Changed<Sequence<T>>)>;

#[derive(Debug, Reflect)]
pub enum TransitionError<T: Behavior> {
    RejectedNext(T),
    NoPrevious,
}

pub type TransitionResult<T> = Result<(), TransitionError<T>>;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TransitionResponse<T: Behavior> {
    result: Option<TransitionResult<T>>,
}

impl<T: Behavior> TransitionResponse<T> {
    pub fn take(mut this: Mut<Self>) -> Option<TransitionResult<T>> {
        // NOTE: Do not call `take()` directly to avoid triggering change detection
        if this.result.is_some() {
            this.result.take()
        } else {
            None
        }
    }

    pub(crate) fn set(&mut self, result: TransitionResult<T>) {
        self.result = Some(result);
    }
}

impl<T: Behavior> Default for TransitionResponse<T> {
    fn default() -> Self {
        Self { result: None }
    }
}
