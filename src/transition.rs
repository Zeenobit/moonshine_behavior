use std::{fmt, mem};

use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_utils::tracing::{debug, error, warn};

use moonshine_kind::{prelude::*, InstanceMutItem};
use moonshine_util::future::{Future, Promise};

use crate::{Behavior, BehaviorEventWriter, Memory};

/// A [`Component`] used to trigger [`Behavior`] transitions.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub enum Transition<B: Behavior> {
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

impl<B: Behavior> Transition<B> {
    pub fn next(next: B) -> (Self, Future<TransitionResult<B>>) {
        let (promise, future) = Promise::start();
        let transition = Self::Next(next, promise);
        (transition, future)
    }

    pub fn is_started(&self) -> bool {
        matches!(self, Self::Started)
    }

    pub fn is_resumed(&self) -> bool {
        matches!(self, Self::Resumed)
    }

    pub fn is_stable(&self) -> bool {
        matches!(self, Self::Stable)
    }

    pub fn is_activated(&self) -> bool {
        self.is_started() || self.is_resumed()
    }

    pub fn is_suspending(&self) -> bool {
        matches!(self, Self::Next { .. } | Self::Previous | Self::Reset)
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
        let old = mem::replace(self, Self::Previous);
        if old.is_suspending() {
            warn!("transition override: {old:?} -> {self:?}");
        }
    }

    pub fn reset(&mut self) {
        let old = mem::replace(self, Self::Reset);
        if old.is_suspending() {
            warn!("transition override: {old:?} -> {self:?}");
        }
    }

    fn take(&mut self) -> Self {
        let mut t = Self::Stable;
        mem::swap(self, &mut t);
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
pub fn transition<B: Behavior>(
    mut query: Query<(InstanceMut<B>, &mut Memory<B>, &mut Transition<B>)>,
    mut events: BehaviorEventWriter<B>,
) {
    for (mut current, memory, mut transition) in &mut query {
        use Transition::*;

        // Do not mutate the transition if stable:
        if matches!(*transition, Stable) {
            continue;
        }

        match transition.take() {
            Next(next, promise) => {
                let result = push(&mut current, next, memory, &mut events);
                if result.is_ok() {
                    if let Some(next) = current.started() {
                        *transition = Next(next, Promise::new());
                    } else {
                        *transition = Started;
                    }
                }
                promise.set(result);
            }
            Previous => {
                if let Some(next) = current.stopped() {
                    let value = push(&mut current, next, memory, &mut events);
                    if value.is_ok() {
                        *transition = Started;
                    }
                } else if pop(&mut current, memory, &mut events) {
                    *transition = Resumed;
                }
            }
            Reset => {
                if reset(&mut current, memory, &mut events) {
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
