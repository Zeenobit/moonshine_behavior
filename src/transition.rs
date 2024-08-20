use std::{fmt, mem};

use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_utils::tracing::{debug, error, warn};

use moonshine_kind::{prelude::*, InstanceMutItem};
use moonshine_util::future::{Future, Promise};

use crate::{Behavior, BehaviorEventWriter, Memory};

use TransitionState::*;

/// A [`Component`] which stores the transition state of a [`Behavior`].
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Transition<B: Behavior>(TransitionState<B>);

impl<B: Behavior> Default for Transition<B> {
    fn default() -> Self {
        Self(TransitionState::default())
    }
}

impl<B: Behavior> Transition<B> {
    pub(crate) fn next(next: B) -> (Self, Future<TransitionResult<B>>) {
        let (promise, future) = Promise::start();
        let state = TransitionState::Next(next, promise);
        (Self(state), future)
    }

    pub fn is_started(&self) -> bool {
        matches!(self.0, Started)
    }

    pub fn is_resumed(&self) -> bool {
        matches!(self.0, Resumed)
    }

    pub fn is_stable(&self) -> bool {
        matches!(self.0, Stable)
    }

    pub fn is_activated(&self) -> bool {
        self.is_started() || self.is_resumed()
    }

    pub fn is_suspending(&self) -> bool {
        matches!(self.0, Next(..) | Previous | Reset)
    }

    pub fn try_start(&mut self, behavior: B) -> Future<TransitionResult<B>> {
        let (new, future) = Self::next(behavior);
        let old = mem::replace(self, new);
        if old.is_suspending() {
            warn!("transition override: {old:?} -> {self:?}");
        }
        future
    }

    pub fn stop(&mut self) {
        let old = Self(mem::replace(&mut self.0, Previous));
        if old.is_suspending() {
            warn!("transition override: {old:?} -> {self:?}");
        }
    }

    pub fn reset(&mut self) {
        let old = Self(mem::replace(&mut self.0, Reset));
        if old.is_suspending() {
            warn!("transition override: {old:?} -> {self:?}");
        }
    }

    fn take(&mut self) -> Self {
        Self(mem::replace(&mut self.0, Stable))
    }

    pub(crate) fn clone(&self) -> Self
    where
        B: Clone,
    {
        use TransitionState::*;
        Self(match &self.0 {
            Stable => Stable,
            Started => Started,
            Resumed => Resumed,
            Next(next, ..) => Next(next.clone(), Promise::new()),
            Previous => Previous,
            Reset => Reset,
        })
    }
}

impl<B: Behavior> fmt::Debug for Transition<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TransitionState::*;
        match &self.0 {
            Stable => write!(f, "Stable"),
            Started => write!(f, "Started"),
            Resumed => write!(f, "Resumed"),
            Next(next, ..) => f.debug_tuple("Next").field(next).finish(),
            Previous => write!(f, "Previous"),
            Reset => write!(f, "Reset"),
        }
    }
}

pub type TransitionResult<B> = Result<(), InvalidTransition<B>>;

#[derive(Debug)]
pub struct InvalidTransition<B: Behavior>(pub B);

/// A [`System`] which triggers [`Behavior`] transitions.
pub fn transition<B: Behavior>(
    mut query: Query<(InstanceMut<B>, &mut Memory<B>, &mut Transition<B>)>,
    mut events: BehaviorEventWriter<B>,
) {
    for (mut current, memory, mut transition) in &mut query {
        use TransitionState::*;

        if transition.is_stable() {
            // Do not mutate the transition if stable
            continue;
        }

        match transition.take().0 {
            Next(next, promise) => {
                let result = push(&mut current, next, memory, &mut events);
                if result.is_ok() {
                    if let Some(next) = current.started() {
                        transition.0 = Next(next, Promise::new());
                    } else {
                        transition.0 = Started;
                    }
                }
                promise.set(result);
            }
            Previous => {
                if let Some(next) = current.stopped() {
                    let value = push(&mut current, next, memory, &mut events);
                    if value.is_ok() {
                        transition.0 = Started;
                    }
                } else if pop(&mut current, memory, &mut events) {
                    transition.0 = Resumed;
                }
            }
            Reset => {
                if reset(&mut current, memory, &mut events) {
                    transition.0 = Resumed;
                }
            }
            Started | Resumed => {
                transition.0 = Stable;
            }
            Stable => unreachable!(),
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
enum TransitionState<B: Behavior> {
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
