pub mod prelude {
    pub use crate::{Behavior, BehaviorMut, BehaviorRef};

    pub use crate::transition::{transition, Next, Previous, Reset, Transition};

    pub use crate::events::{TransitionEvent, TransitionEvents};
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
use std::ops::{Deref, DerefMut, Index};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Tick;
use bevy_ecs::{prelude::*, query::QueryData};
use bevy_reflect::prelude::*;
use bevy_utils::prelude::*;
use bevy_utils::tracing::{debug, warn};
use events::TransitionEventsMut;
use moonshine_kind::prelude::*;

use crate::events::TransitionEvent;

use self::transition::*;

pub trait Behavior: 'static + Debug + Send + Sync {
    /// Called when an interrupt is requested.
    ///
    /// If this returns `true`, the current behavior will stop to allow the next behavior to start.
    /// Note that the initial behavior is never allowed to yield.
    fn filter_yield(&self, next: &Self) -> bool {
        match_next! {
            self => next,
            _ => _,
        }
    }

    /// Called before a new behavior is started.
    ///
    /// If this returns `false`, the transition fails.
    /// See [`BehaviorEvents::error`](crate::events::BehaviorEvents) for details on how to handle transition failures.
    fn filter_next(&self, next: &Self) -> bool {
        match_next! {
            self => next,
            _ => _,
        }
    }

    /// Called after a behavior is paused.
    ///
    /// If this returns `false`, the paused behavior will be stopped immediatedly and discarded.
    /// Note that no [`pause`](crate::events::BehaviorEvents::pause) event will be sent in this case.
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
    /// Returns the current [`Behavior`] state.
    pub fn current(&self) -> &T {
        &self.current
    }

    /// Returns the index associated with the current [`Behavior`] state.
    ///
    /// # Usage
    ///
    /// Each behavior state is associated with an index which corresponds to their position in the stack.
    ///
    /// The current behavior is always at the top of the stack.
    /// The initial behavior always has the index of `0``.
    ///
    /// This index may be used to identify the exact unique behavior state when multiple similar states are in the stack.
    pub fn index(&self) -> usize {
        self.memory.stack.len()
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

impl<T: Behavior + Component> Index<usize> for BehaviorRefItem<'_, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index == self.memory.stack.len() {
            self.current()
        } else {
            &self.memory[index]
        }
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
    pub fn interrupt_start(&mut self, next: T) {
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

    fn push(&mut self, instance: Instance<T>, mut next: T, events: &mut TransitionEventsMut<T>) {
        if self.filter_next(&next) {
            let previous = {
                swap(self.current.as_mut(), &mut next);
                next
            };
            let index = self.memory.len();
            if previous.is_resumable() {
                let new_index = index + 1;
                debug!(
                    "{instance:?}: {previous:?} (#{index}) -> {:?} (#{new_index})",
                    *self.current
                );
                events.send(TransitionEvent::Pause { instance, index });
                events.send(TransitionEvent::Start {
                    instance,
                    index: new_index,
                });
                self.memory.push(previous);
            } else {
                events.send(TransitionEvent::Start { instance, index });
                events.send(TransitionEvent::Stop {
                    instance,
                    behavior: previous,
                });
            }
        } else {
            warn!(
                "{instance:?}: transition {:?} -> {next:?} is not allowed",
                *self.current
            );
            events.send(TransitionEvent::Error {
                instance,
                error: TransitionError::RejectedNext(next),
            });
        }
    }

    fn interrupt(&mut self, instance: Instance<T>, next: T, events: &mut TransitionEventsMut<T>) {
        while self.filter_yield(&next) && !self.memory.is_empty() {
            let index = self.memory.len();
            if let Some(mut previous) = self.memory.pop() {
                let previous_index = self.memory.len();
                debug!(
                    "{instance:?}: {:?} (#{index}) -> {previous:?} (#{previous_index})",
                    *self.current
                );
                let previous = {
                    swap(self.current.as_mut(), &mut previous);
                    previous
                };
                events.send(TransitionEvent::Stop {
                    instance,
                    behavior: previous,
                });
            }
        }

        self.push(instance, next, events);
    }

    fn pop(&mut self, instance: Instance<T>, events: &mut TransitionEventsMut<T>) {
        let index = self.memory.len();
        if let Some(mut previous) = self.memory.pop() {
            let previous_index = self.memory.len();
            debug!(
                "{instance:?}: {:?} (#{index}) -> {previous:?} (#{previous_index})",
                *self.current
            );
            let behavior = {
                swap(self.current.as_mut(), &mut previous);
                previous
            };
            events.send(TransitionEvent::Resume {
                instance,
                index: previous_index,
            });
            events.send(TransitionEvent::Stop { instance, behavior });
        } else {
            warn!(
                "{instance:?}: transition {:?} -> None is not allowed",
                *self.current
            );
            events.send(TransitionEvent::Error {
                instance,
                error: TransitionError::NoPrevious,
            });
        }
    }

    fn clear(&mut self, instance: Instance<T>, events: &mut TransitionEventsMut<T>) {
        while self.memory.len() > 1 {
            let previous = self.memory.pop().unwrap();
            events.send(TransitionEvent::Stop {
                instance,
                behavior: previous,
            });
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
