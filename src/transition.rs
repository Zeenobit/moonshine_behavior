use std::{fmt, mem};

use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_utils::tracing::{debug, error, warn};
use moonshine_util::future::{Future, Promise};

use crate::{Behavior, BehaviorEvents, Memory};

/// A [`Component`] used to trigger [`Behavior`] transitions.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub enum Transition<B: Behavior> {
    #[default]
    Stable,
    Started,
    Resumed,
    #[reflect(ignore)]
    Next(B, #[reflect(ignore)] Promise<TransitionResult<B>>),
    #[reflect(ignore)]
    Previous,
    #[reflect(ignore)]
    Reset,
}

impl<B: Behavior> Transition<B> {
    pub fn next(next: B) -> (Self, Future<TransitionResult<B>>) {
        let (promise, future) = Promise::start();
        let transition = Self::Next(next, promise);
        (transition, future)
    }

    pub fn previous() -> Self {
        Self::Previous
    }

    pub fn reset() -> Self {
        Self::Reset
    }

    fn take(&mut self) -> Self {
        let mut t = Self::Stable;
        std::mem::swap(self, &mut t);
        t
    }

    pub(crate) fn clone(&self) -> Self
    where
        B: Clone,
    {
        use Transition::*;
        match self {
            Stable => Stable,
            Started => Started,
            Resumed => Resumed,
            Next(next, _) => Next(next.clone(), Promise::new()),
            Previous => Previous,
            Reset => Reset,
        }
    }
}

impl<B: Behavior> fmt::Debug for Transition<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Transition::*;
        match self {
            Stable => write!(f, "Stable"),
            Started => write!(f, "Started"),
            Resumed => write!(f, "Resumed"),
            Next(arg0, _) => f.debug_tuple("Next").field(arg0).finish(),
            Previous => write!(f, "Previous"),
            Reset => write!(f, "Reset"),
        }
    }
}

pub type TransitionResult<B> = Result<(), InvalidTransition<B>>;

#[derive(Debug)]
pub struct InvalidTransition<B: Behavior>(pub B);

/// A [`System`] which triggers [`Behavior`] transitions.
#[allow(clippy::type_complexity)]
pub fn transition<B: Behavior>(
    mut query: Query<(Entity, &mut B, &mut Memory<B>, &mut Transition<B>)>,
    mut events: BehaviorEvents<B>,
) {
    for (entity, mut current, memory, mut transition) in &mut query {
        use Transition::*;

        // Do not mutate the transition if stable:
        if matches!(*transition, Stable) {
            continue;
        }

        match transition.take() {
            Next(next, promise) => {
                let value = push(entity, next, &mut current, memory, &mut events);
                if value.is_ok() {
                    if let Some(next) = current.started() {
                        *transition = Next(next, Promise::new());
                    } else {
                        *transition = Started;
                    }
                }
                promise.set(value);
            }
            Previous => {
                if let Some(next) = current.stopped() {
                    let value = push(entity, next, &mut current, memory, &mut events);
                    if value.is_ok() {
                        *transition = Started;
                    }
                } else if pop(entity, current, memory, &mut events) {
                    *transition = Resumed;
                }
            }
            Reset => {
                if reset(entity, current, memory, &mut events) {
                    *transition = Resumed;
                }
            }
            Started | Resumed => {
                *transition = Stable;
            }
            Stable => unreachable!(),
        }
    }
}

fn push<B: Behavior>(
    entity: Entity,
    mut next: B,
    current: &mut Mut<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut BehaviorEvents<B>,
) -> TransitionResult<B> {
    if current.allows_next(&next) {
        debug!("{entity:?}: {:?} -> {next:?}", *current);
        let behavior = {
            mem::swap(current.as_mut(), &mut next);
            next
        };
        if behavior.is_resumable() {
            events.send_paused(entity);
            memory.push(behavior);
        } else {
            events.send_stopped(entity, behavior);
        }
        events.send_started(entity);
        Ok(())
    } else {
        warn!("{entity:?}: {:?} -> {next:?} is not allowed", *current);
        Err(InvalidTransition(next))
    }
}

fn pop<B: Behavior>(
    entity: Entity,
    mut current: Mut<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut BehaviorEvents<B>,
) -> bool {
    if let Some(mut next) = memory.pop() {
        debug!("{entity:?}: {:?} -> {next:?}", *current);
        let behavior = {
            mem::swap(current.as_mut(), &mut next);
            next
        };
        events.send_resumed(entity);
        events.send_stopped(entity, behavior);
        true
    } else {
        error!("{entity:?}: {:?} -> None is not allowed", *current);
        false
    }
}

fn reset<B: Behavior>(
    entity: Entity,
    mut current: Mut<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut BehaviorEvents<B>,
) -> bool {
    while memory.len() > 1 {
        let behavior = memory.pop().unwrap();
        events.send_stopped(entity, behavior);
    }

    if let Some(mut next) = memory.pop() {
        debug!("{entity:?}: {:?} -> {next:?}", *current);
        let behavior = {
            mem::swap(current.as_mut(), &mut next);
            next
        };
        events.send_resumed(entity);
        events.send_stopped(entity, behavior);
        true
    } else {
        warn!("{entity:?}: {:?} -> {:?} is redundant", *current, *current);
        false
    }
}
