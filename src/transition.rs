use std::{fmt, mem};

use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_utils::tracing::{debug, error, warn};

use moonshine_kind::{prelude::*, InstanceMutItem};
use moonshine_util::future::{Future, Promise};

use crate::{Behavior, BehaviorEventWriter, Memory};

use Transition::*;

/// A [`Component`] which controls the state of a [`Behavior`].
///
/// Insert this with your behavior component (`#[require(Controller<B>)]` works too!) to control and query its state.
#[derive(Component, Reflect)]
#[require(Memory::<B>)]
#[reflect(Component)]
pub struct Controller<B: Behavior> {
    transition: Transition<B>,
}

impl<B: Behavior> Default for Controller<B> {
    fn default() -> Self {
        Self {
            transition: Transition::default(),
        }
    }
}

impl<B: Behavior> Clone for Controller<B> {
    fn clone(&self) -> Self {
        if self.is_started() {
            Self {
                transition: Started,
            }
        } else {
            panic!("cannot clone transition after initialization: {self:?}")
        }
    }
}

impl<B: Behavior> Controller<B> {
    pub fn next(next: B) -> Self {
        Self::next_internal(next).0
    }

    pub(crate) fn next_internal(next: B) -> (Self, Future<TransitionResult<B>>) {
        let (promise, future) = Promise::start();
        let transition = Transition::Next(next, promise);
        (Self { transition }, future)
    }

    pub fn is_started(&self) -> bool {
        matches!(self.transition, Started)
    }

    pub fn is_resumed(&self) -> bool {
        matches!(self.transition, Resumed)
    }

    pub fn is_stable(&self) -> bool {
        matches!(self.transition, Stable)
    }

    pub fn is_activated(&self) -> bool {
        self.is_started() || self.is_resumed()
    }

    pub fn is_suspending(&self) -> bool {
        matches!(self.transition, Next(..) | Previous | Reset)
    }

    pub fn try_start(&mut self, behavior: B) -> Future<TransitionResult<B>> {
        let (new, future) = Self::next_internal(behavior);
        let old = mem::replace(self, new);
        if old.is_suspending() {
            warn!("transition override: {old:?} -> {self:?}");
        }
        future
    }

    pub fn stop(&mut self) {
        let old = Self {
            transition: mem::replace(&mut self.transition, Previous),
        };
        if old.is_suspending() {
            warn!("transition override: {old:?} -> {self:?}");
        }
    }

    pub fn reset(&mut self) {
        let old = Self {
            transition: mem::replace(&mut self.transition, Reset),
        };
        if old.is_suspending() {
            warn!("transition override: {old:?} -> {self:?}");
        }
    }

    fn take(&mut self) -> Self {
        Self {
            transition: mem::replace(&mut self.transition, Stable),
        }
    }

    pub(crate) fn clone(&self) -> Self
    where
        B: Clone,
    {
        use Transition::*;
        Self {
            transition: match &self.transition {
                Stable => Stable,
                Started => Started,
                Resumed => Resumed,
                Next(next, ..) => Next(next.clone(), Promise::new()),
                Previous => Previous,
                Reset => Reset,
            },
        }
    }
}

impl<B: Behavior> fmt::Debug for Controller<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Transition::*;
        match &self.transition {
            Stable => write!(f, "Transition::<{}>::Stable", B::debug_name()),
            Started => write!(f, "Transition::<{}>::Started", B::debug_name()),
            Resumed => write!(f, "Transition::<{}>::Resumed", B::debug_name()),
            Next(next, ..) => f
                .debug_tuple(format!("Transition::<{}>::Next", B::debug_name()).as_str())
                .field(next)
                .finish(),
            Previous => write!(f, "Transition::<{}>::Previous", B::debug_name()),
            Reset => write!(f, "Transition::<{}>::Reset", B::debug_name()),
        }
    }
}

pub type TransitionResult<B> = Result<(), InvalidTransition<B>>;

#[derive(Debug)]
pub struct InvalidTransition<B: Behavior>(pub B);

/// A [`System`] which triggers [`Behavior`] transitions.
pub fn transition<B: Behavior>(
    mut query: Query<(InstanceMut<B>, &mut Memory<B>, &mut Controller<B>)>,
    mut events: BehaviorEventWriter<B>,
) {
    for (mut current, memory, mut transition) in &mut query {
        use Transition::*;

        if transition.is_stable() {
            // Do not mutate the transition if stable
            continue;
        }

        match transition.take().transition {
            Next(next, promise) => {
                let result = push(&mut current, next, memory, &mut events);
                if result.is_ok() {
                    if let Some(next) = current.started() {
                        transition.transition = Next(next, Promise::new());
                    } else {
                        transition.transition = Started;
                    }
                }
                promise.set(result);
            }
            Previous => {
                if let Some(next) = current.stopped() {
                    let value = push(&mut current, next, memory, &mut events);
                    if value.is_ok() {
                        transition.transition = Started;
                    }
                } else if pop(&mut current, memory, &mut events) {
                    transition.transition = Resumed;
                }
            }
            Reset => {
                if reset(&mut current, memory, &mut events) {
                    transition.transition = Resumed;
                }
            }
            Started | Resumed => {
                transition.transition = Stable;
            }
            Stable => unreachable!(),
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum Transition<B: Behavior> {
    Stable,
    #[default]
    Started,
    Resumed,
    #[reflect(ignore)]
    Next(B, #[reflect(ignore)] Promise<TransitionResult<B>>),
    #[reflect(ignore)]
    Previous,
    #[reflect(ignore)]
    Reset,
}

fn push<B: Behavior>(
    current: &mut InstanceMutItem<B>,
    mut next: B,
    mut memory: Mut<Memory<B>>,
    events: &mut BehaviorEventWriter<B>,
) -> TransitionResult<B> {
    if current.allows_next(&next) {
        debug!("{current:?}: {:?} -> {next:?}", **current);
        let behavior = {
            mem::swap(current.as_mut(), &mut next);
            next
        };
        if behavior.is_resumable() {
            events.send_paused(current.instance());
            memory.push(behavior);
        } else {
            events.send_stopped(current.instance(), behavior);
        }
        events.send_started(current.instance());
        Ok(())
    } else {
        warn!("{current:?}: {:?} -> {next:?} is not allowed", **current);
        Err(InvalidTransition(next))
    }
}

fn pop<B: Behavior>(
    current: &mut InstanceMutItem<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut BehaviorEventWriter<B>,
) -> bool {
    if let Some(mut next) = memory.pop() {
        debug!("{current:?}: {:?} -> {next:?}", **current);
        let behavior = {
            mem::swap(current.as_mut(), &mut next);
            next
        };
        events.send_resumed(current.instance());
        events.send_stopped(current.instance(), behavior);
        true
    } else {
        error!("{current:?}: {:?} -> None is not allowed", **current);
        false
    }
}

fn reset<B: Behavior>(
    current: &mut InstanceMutItem<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut BehaviorEventWriter<B>,
) -> bool {
    while memory.len() > 1 {
        let behavior = memory.pop().unwrap();
        events.send_stopped(current.instance(), behavior);
    }

    if let Some(mut next) = memory.pop() {
        debug!("{current:?}: {:?} -> {next:?}", **current);
        let behavior = {
            mem::swap(current.as_mut(), &mut next);
            next
        };
        events.send_resumed(current.instance());
        events.send_stopped(current.instance(), behavior);
        true
    } else {
        warn!(
            "{current:?}: {:?} -> {:?} is redundant",
            **current, **current
        );
        false
    }
}
