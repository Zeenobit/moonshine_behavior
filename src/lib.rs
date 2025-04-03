pub mod prelude {
    pub use crate::{Behavior, BehaviorMut, BehaviorRef};

    pub use crate::transition::{transition, Next, Previous, Reset, Transition};

    pub use crate::events::BehaviorEvents;
    pub use crate::plugin::BehaviorPlugin;

    pub use crate::sequence::Sequence;

    pub use crate::match_next;
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
use bevy_ecs::component::Tick;
use bevy_ecs::{prelude::*, query::QueryData};
use bevy_reflect::prelude::*;
use bevy_utils::prelude::*;
use bevy_utils::tracing::{debug, warn};
use events::BehaviorEventsMut;
use moonshine_kind::prelude::*;

use self::transition::*;

pub trait Behavior: 'static + Debug + Send + Sync {
    fn filter_yield(&self, next: &Self) -> bool {
        match_next! {
            self => next,
            _ => _,
        }
    }

    fn filter_next(&self, next: &Self) -> bool {
        match_next! {
            self => next,
            _ => _,
        }
    }

    fn is_resumable(&self) -> bool {
        match self {
            _ => true,
        }
    }
}

#[derive(QueryData)]
pub struct BehaviorRef<T: Behavior + Component> {
    current: Ref<'static, T>,
    memory: &'static Memory<T>,
    transition: &'static Transition<T>,
}

impl<T: Behavior + Component> BehaviorRefItem<'_, T> {
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

impl<T: Behavior + Component> Deref for BehaviorRefItem<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.current()
    }
}

impl<T: Behavior + Component> AsRef<T> for BehaviorRefItem<'_, T> {
    fn as_ref(&self) -> &T {
        &self.current
    }
}

impl<T: Behavior + Component> DetectChanges for BehaviorRefItem<'_, T> {
    fn is_added(&self) -> bool {
        self.current.is_added()
    }

    fn is_changed(&self) -> bool {
        self.current.is_changed()
    }

    fn last_changed(&self) -> Tick {
        self.current.last_changed()
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct BehaviorMut<T: Behavior + Component> {
    current: Mut<'static, T>,
    memory: &'static mut Memory<T>,
    transition: &'static mut Transition<T>,
}

impl<T: Behavior + Component> BehaviorMutReadOnlyItem<'_, T> {
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

impl<T: Behavior + Component> Deref for BehaviorMutReadOnlyItem<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.current()
    }
}

impl<T: Behavior + Component> AsRef<T> for BehaviorMutReadOnlyItem<'_, T> {
    fn as_ref(&self) -> &T {
        self.current.as_ref()
    }
}

impl<T: Behavior + Component> DetectChanges for BehaviorMutReadOnlyItem<'_, T> {
    fn is_added(&self) -> bool {
        self.current.is_added()
    }

    fn is_changed(&self) -> bool {
        self.current.is_changed()
    }

    fn last_changed(&self) -> Tick {
        self.current.last_changed()
    }
}

impl<T: Behavior + Component> BehaviorMutItem<'_, T> {
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
}

impl<T: Behavior + Component> BehaviorMutItem<'_, T> {
    /// Starts the given `next` behavior.
    ///
    /// This operation pushes the current behavior onto the stack and inserts the new behavior.
    ///
    /// Note that this will fail if the current behavior rejects `next` through [`Behavior::filter_next`].
    /// See [`BehaviorEvents::error`](crate::events::BehaviorEvents) for details on how to handle transition failures.
    pub fn start(&mut self, next: T) {
        self.set_transition(Next(next));
    }

    /// Interrupts the current behavior and starts the given `next` behavior.
    ///
    /// This operation stops all behaviors which yield to the new behavior and pushes it onto the stack.
    /// See [`Behavior::filter_yield`] for details on how to define how states yield to each other.
    ///
    /// The initial behavior is never allowed to yield.
    ///
    /// Note that this will fail if the first non-yielding behavior rejects `next` through [`Behavior::filter_next`].
    /// See [`BehaviorEvents::error`](crate::events::BehaviorEvents) for details on how to handle transition failures.
    pub fn start_interrupt(&mut self, next: T) {
        self.set_transition(Interrupt(next));
    }

    /// Stops the current behavior.
    ///
    /// This operation pops the current behavior off the stack and resumes the previous behavior.
    ///
    /// Note that this will fail if the current behavior is the initial behavior.
    /// See [`BehaviorEvents::error`](crate::events::BehaviorEvents) for details on how to handle transition failures.
    pub fn stop(&mut self) {
        self.set_transition(Previous);
    }

    /// Stops the current and all previous behaviors and resumes the initial behavior.
    ///
    /// This operation clears the stack and resumes the initial behavior. It can never fail.
    /// If the stack is empty (i.e. initial behavior), it does nothing.
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
            //self.set_result(Ok(()));
        } else {
            warn!(
                "{instance:?}: transition {:?} -> {next:?} is not allowed",
                *self.current
            );
            events.error(instance, TransitionError::RejectedNext(next));
        }
    }

    fn interrupt(&mut self, instance: Instance<T>, next: T, events: &mut BehaviorEventsMut<T>) {
        while self.filter_yield(&next) && !self.memory.is_empty() {
            if let Some(mut previous) = self.memory.pop() {
                debug!("{instance:?}: {:?} -> {previous:?}", *self.current);
                let previous = {
                    swap(self.current.as_mut(), &mut previous);
                    previous
                };
                events.stop(instance, previous);
            }
        }

        self.push(instance, next, events);
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
        } else {
            warn!(
                "{instance:?}: transition {:?} -> None is not allowed",
                *self.current
            );
            events.error(instance, TransitionError::NoPrevious);
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

impl<T: Behavior + Component> Deref for BehaviorMutItem<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.current()
    }
}

impl<T: Behavior + Component> DerefMut for BehaviorMutItem<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.current_mut()
    }
}

impl<T: Behavior + Component> DetectChanges for BehaviorMutItem<'_, T> {
    fn is_added(&self) -> bool {
        self.current.is_added()
    }

    fn is_changed(&self) -> bool {
        self.current.is_changed()
    }

    fn last_changed(&self) -> Tick {
        self.current.last_changed()
    }
}

impl<T: Behavior + Component> DetectChangesMut for BehaviorMutItem<'_, T> {
    type Inner = T;

    fn set_changed(&mut self) {
        self.current.set_changed()
    }

    fn set_last_changed(&mut self, last_changed: Tick) {
        self.current.set_last_changed(last_changed)
    }

    fn bypass_change_detection(&mut self) -> &mut Self::Inner {
        self.current.bypass_change_detection()
    }
}

impl<T: Behavior + Component> AsRef<T> for BehaviorMutItem<'_, T> {
    fn as_ref(&self) -> &T {
        self.current.as_ref()
    }
}

impl<T: Behavior + Component> AsMut<T> for BehaviorMutItem<'_, T> {
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

/// A convenient macro for implementation of [`Behavior::filter_next`].
///
/// # Usage
/// For any given pair of states `curr` and `next`, this match expands into a match arm such that:
/// ```rust,ignore
/// match curr {
///     From => matches!(next, To)
/// }
/// ```
/// where `From` is the current possible state, and `To` is the next allowed state.
///
/// # Example
///
/// ```rust
/// # use bevy::prelude::*;
/// # use moonshine_behavior::prelude::*;
/// #[derive(Component, Debug)]
/// enum Soldier {
///     Idle,
///     Crouch,
///     Fire,
///     Sprint,
///     Jump,
/// }
///
/// impl Behavior for Soldier {
///     fn filter_next(&self, next: &Self) -> bool {
///         use Soldier::*;
///         match_next! {
///             self => next,
///             Idle => Crouch | Sprint | Fire | Jump,
///             Crouch => Sprint | Fire,
///             Sprint => Jump,
///         }
///     }
/// }
///
/// # assert!(Soldier::Idle.filter_next(&Soldier::Crouch));
/// # assert!(!Soldier::Sprint.filter_next(&Soldier::Fire));
/// ```
#[macro_export]
macro_rules! match_next {
    { $curr:ident => $next:ident, $($from:pat => $to:pat),* $(,)? } => {
        match $curr {
            $(
                $from => matches!($next, $to),
            )*
            #[allow(unreachable_patterns)]
            _ => false,
        }
    };
}
