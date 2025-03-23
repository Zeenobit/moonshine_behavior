pub mod prelude {
    pub use crate::{Behavior, BehaviorMut, BehaviorRef};

    pub use crate::transition::{transition, Next, Previous, Reset, Transition};

    pub use crate::events::BehaviorEvents;
    pub use crate::plugin::BehaviorPlugin;

    pub use crate::sequence::Sequence;
}

pub mod events;
pub mod plugin;
pub mod sequence;
pub mod transition;

#[cfg(test)]
mod tests;

use std::fmt::Debug;
use std::mem::{replace, swap};
use std::ops::{Deref, DerefMut};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, query::QueryData};
use bevy_reflect::prelude::*;
use bevy_utils::prelude::*;
use bevy_utils::tracing::{debug, warn};
use events::BehaviorEventsMut;
use moonshine_kind::prelude::*;

use self::transition::*;

pub trait Behavior: Component + Debug {
    fn filter_next(&self, next: &Self) -> bool {
        match (self, next) {
            _ => true,
        }
    }

    fn is_resumable(&self) -> bool {
        match self {
            _ => true,
        }
    }
}

#[derive(QueryData)]
pub struct BehaviorRef<T: Behavior> {
    current: &'static T,
    memory: &'static Memory<T>,
    transition: &'static Transition<T>,
}

impl<T: Behavior> BehaviorRefItem<'_, T> {
    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn previous(&self) -> Option<&T> {
        self.memory.last()
    }

    pub fn has_transition(&self) -> bool {
        !self.transition.is_none()
    }
}

impl<T: Behavior> Deref for BehaviorRefItem<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.current()
    }
}

impl<T: Behavior> AsRef<T> for BehaviorRefItem<'_, T> {
    fn as_ref(&self) -> &T {
        self.current
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct BehaviorMut<T: Behavior> {
    current: Mut<'static, T>,
    memory: &'static mut Memory<T>,
    transition: &'static mut Transition<T>,
    response: Option<&'static mut TransitionResponse<T>>,
}

impl<T: Behavior> BehaviorMutReadOnlyItem<'_, T> {
    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn previous(&self) -> Option<&T> {
        self.memory.last()
    }

    pub fn has_transition(&self) -> bool {
        !self.transition.is_none()
    }
}

impl<T: Behavior> Deref for BehaviorMutReadOnlyItem<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.current()
    }
}

impl<T: Behavior> AsRef<T> for BehaviorMutReadOnlyItem<'_, T> {
    fn as_ref(&self) -> &T {
        self.current.as_ref()
    }
}

impl<T: Behavior> BehaviorMutItem<'_, T> {
    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn current_mut(&mut self) -> &mut T {
        self.current.as_mut()
    }

    pub fn previous(&self) -> Option<&T> {
        self.memory.last()
    }

    pub fn has_transition(&self) -> bool {
        !self.transition.is_none()
    }

    pub fn start(&mut self, behavior: T) {
        self.set_transition(Next(behavior));
    }

    pub fn stop(&mut self) {
        self.set_transition(Previous);
    }

    pub fn reset(&mut self) {
        self.set_transition(Reset);
    }

    fn set_transition(&mut self, transition: Transition<T>) {
        let previous = replace(self.transition.as_mut(), transition);
        if !previous.is_none() {
            warn!(
                "transition override: {previous:?} -> {:?}",
                *self.transition
            );
        }
    }

    fn set_result(&mut self, result: TransitionResult<T>) {
        if let Some(response) = self.response.as_mut() {
            response.set(result);
        }
    }

    fn push(&mut self, instance: Instance<T>, mut next: T, events: &mut BehaviorEventsMut<T>) {
        if self.filter_next(&next) {
            let previous = {
                swap(self.current.as_mut(), &mut next);
                next
            };
            debug!("{instance:?}: {previous:?} -> {:?}", *self.current);
            if previous.is_resumable() {
                events.pause(instance);
                self.memory.push(previous);
            } else {
                events.stop(instance, previous);
            }
            events.start(instance);
            self.set_result(Ok(()));
        } else {
            warn!(
                "{instance:?}: transition {:?} -> {next:?} is not allowed",
                *self.current
            );
            self.set_result(Err(TransitionError::RejectedNext(next)));
        }
    }

    fn pop(&mut self, instance: Instance<T>, events: &mut BehaviorEventsMut<T>) {
        if let Some(mut previous) = self.memory.pop() {
            debug!("{instance:?}: {:?} -> {previous:?}", *self.current);
            let previous = {
                swap(self.current.as_mut(), &mut previous);
                previous
            };
            events.resume(instance);
            events.stop(instance, previous);
            self.set_result(Ok(()));
        } else {
            warn!(
                "{instance:?}: transition {:?} -> None is not allowed",
                *self.current
            );
            self.set_result(Err(TransitionError::NoPrevious));
        }
    }

    fn clear(&mut self, instance: Instance<T>, events: &mut BehaviorEventsMut<T>) {
        while self.memory.len() > 1 {
            let previous = self.memory.pop().unwrap();
            events.stop(instance, previous);
        }

        self.pop(instance, events);
    }
}

impl<T: Behavior> Deref for BehaviorMutItem<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.current()
    }
}

impl<T: Behavior> DerefMut for BehaviorMutItem<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.current_mut()
    }
}

impl<T: Behavior> AsRef<T> for BehaviorMutItem<'_, T> {
    fn as_ref(&self) -> &T {
        self.current.as_ref()
    }
}

impl<T: Behavior> AsMut<T> for BehaviorMutItem<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        self.current.as_mut()
    }
}

#[derive(Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
struct Memory<T: Behavior> {
    stack: Vec<T>,
}

impl<T: Behavior> Default for Memory<T> {
    fn default() -> Self {
        Self { stack: default() }
    }
}
