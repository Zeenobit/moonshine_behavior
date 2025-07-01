#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

pub mod prelude {
    pub use crate::{Behavior, BehaviorMut, BehaviorRef};
    pub use moonshine_kind::{Instance, InstanceCommands};

    pub use crate::transition::{
        transition, Next, Previous, Reset, Transition, TransitionSequence,
    };

    pub use crate::events::{OnPause, OnResume, OnStart, OnStop};
    pub use crate::plugin::BehaviorPlugin;

    pub use crate::match_next;
}

pub mod events;
pub mod plugin;
pub mod transition;

#[cfg(test)]
mod tests;

use std::fmt::Debug;
use std::mem::{replace, swap};
use std::ops::{Deref, DerefMut, Index, IndexMut};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::change_detection::MaybeLocation;
use bevy_ecs::component::{Components, Mutable, Tick};
use bevy_ecs::{prelude::*, query::QueryData};
use bevy_log::prelude::*;
use bevy_reflect::prelude::*;
use moonshine_kind::prelude::*;

use crate::events::{OnError, OnPause, OnResume, OnStart, OnStop};

use self::transition::*;

pub trait Behavior: Component<Mutability = Mutable> + Debug + Sized {
    /// Called when an interrupt is requested.
    ///
    /// If this returns `true`, the current behavior will stop to allow the next behavior to start.
    /// The initial behavior is never allowed to yield.
    fn filter_yield(&self, next: &Self) -> bool {
        match_next! {
            self => next,
            _ => _,
        }
    }

    /// Called before a new behavior is started.
    ///
    /// If this returns `false`, the transition fails.
    /// See [`Error`](crate::events::BehaviorEvent) for details on how to handle transition failures.
    fn filter_next(&self, next: &Self) -> bool {
        match_next! {
            self => next,
            _ => _,
        }
    }

    /// Called after a behavior is paused.
    ///
    /// If this returns `false`, the paused behavior will be stopped immediatedly and discarded.
    /// No [`Pause`](crate::events::BehaviorEvent) event will be sent in this case.
    fn is_resumable(&self) -> bool {
        match self {
            _ => true,
        }
    }

    /// Called during [`transition`](transition::transition) just after the behavior is started.
    fn on_start(&self, _previous: Option<&Self>, _commands: InstanceCommands<Self>) {}

    /// Called during [`transition`](transition::transition) just after the behavior is paused.
    fn on_pause(&self, _current: &Self, _commands: InstanceCommands<Self>) {}

    /// Called during [`transition`](transition::transition) just after the behavior is resumed.
    fn on_resume(&self, _previous: &Self, _commands: InstanceCommands<Self>) {}

    /// Called during [`transition`](transition::transition) just after the behavior is stopped.
    fn on_stop(&self, _current: &Self, _commands: InstanceCommands<Self>) {}

    /// Called during [`transition`](transition::transition) just after the behavior is started *or* resumed.
    fn on_activate(&self, _previous: Option<&Self>, _commands: InstanceCommands<Self>) {}

    /// Called during [`transition`](transition::transition) just after the behavior is paused *or* stopped.
    fn on_suspend(&self, _current: &Self, _commands: InstanceCommands<Self>) {}
}

#[doc(hidden)]
trait BehaviorHooks: Behavior {
    fn invoke_start(&self, previous: Option<&Self>, mut commands: InstanceCommands<Self>) {
        self.on_start(previous, commands.reborrow());
        self.on_activate(previous, commands);
    }

    fn invoke_pause(&self, current: &Self, mut commands: InstanceCommands<Self>) {
        self.on_suspend(current, commands.reborrow());
        self.on_pause(current, commands);
    }

    fn invoke_resume(&self, previous: &Self, mut commands: InstanceCommands<Self>) {
        self.on_resume(previous, commands.reborrow());
        self.on_activate(Some(previous), commands);
    }

    fn invoke_stop(&self, current: &Self, mut commands: InstanceCommands<Self>) {
        self.on_suspend(current, commands.reborrow());
        self.on_stop(current, commands);
    }
}

impl<T: Behavior> BehaviorHooks for T {}

/// Query data for a [`Behavior`].
///
/// # Usage
///
/// This provides a read-only reference to the current behavior state and all previous states in the stack.
///
/// Additionally, it provides methods to check for pending transitions ([`has_transition`](BehaviorRefItem::has_transition)),
/// active transition sequences ([`has_sequence`](BehaviorRefItem::has_sequence)) and the current
/// behavior index ([`index`](BehaviorRefItem::index)).
#[derive(QueryData)]
pub struct BehaviorRef<T: Behavior> {
    current: Ref<'static, T>,
    memory: &'static Memory<T>,
    transition: &'static Transition<T>,
    sequence: Has<TransitionSequence<T>>,
}

impl<T: Behavior> BehaviorRef<T> {
    /// Creates a new [`BehaviorRef`] item from an [`EntityRef`].
    pub fn from_entity(entity: EntityRef) -> Option<BehaviorRefItem<T>> {
        Some(BehaviorRefItem {
            current: entity.get_ref::<T>()?,
            memory: entity.get::<Memory<T>>()?,
            transition: entity.get::<Transition<T>>()?,
            sequence: entity.contains::<TransitionSequence<T>>(),
        })
    }
}

impl<T: Behavior> BehaviorRefItem<'_, T> {
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
        self.memory.len()
    }

    /// Returns the previous [`Behavior`] state in the stack.
    ///
    /// # Usage
    ///
    /// Note that this is **NOT** the previously active state.
    /// Instead, this is the previous state which was active before the current one was started.
    ///
    /// To access the previously active state, handle [`Stop`](crate::events::BehaviorEvent::Stop) instead.
    pub fn previous(&self) -> Option<&T> {
        self.memory.last()
    }

    /// Returns an iterator over all [`Behavior`] states in the stack, including the current one.
    ///
    /// The iterator is ordered from the initial behavior (index = 0) to the current one.
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.memory.iter().chain(std::iter::once(self.current()))
    }

    /// Returns `true` if there is any pending [`Transition`] for this [`Behavior`].
    ///
    /// # Usage
    ///
    /// By design, only one transition is allowed per [`transition`](crate::transition::transition) cycle.
    ///
    /// The only exception to this rule is if the behavior is interrupted or reset where multiple states
    /// may be stopped within a single cycle.
    ///
    /// If a transition is requested while another is pending, it would be overriden.
    /// The transition helper methods [`start`](BehaviorMutItem::start), [`interrupt_start`](BehaviorMutItem::interrupt_start),
    /// [`stop`](BehaviorMutItem::stop) and [`reset`](BehaviorMutItem::reset) all trigger a warning in this case.
    ///
    /// Because of this, this method is useful to avoid unintentional transition overrides.
    pub fn has_transition(&self) -> bool {
        !self.transition.is_none()
    }

    /// Returns `true` if there is any [`TransitionSequence`] running on this [`Behavior`].
    ///
    /// This is useful to allow transition sequences to finish before starting a new behavior.
    pub fn has_sequence(&self) -> bool {
        self.sequence
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
        &self.current
    }
}

impl<T: Behavior> DetectChanges for BehaviorRefItem<'_, T> {
    fn is_added(&self) -> bool {
        self.current.is_added()
    }

    fn is_changed(&self) -> bool {
        self.current.is_changed()
    }

    fn last_changed(&self) -> Tick {
        self.current.last_changed()
    }

    fn added(&self) -> Tick {
        self.current.added()
    }

    fn changed_by(&self) -> MaybeLocation {
        self.current.changed_by()
    }
}

impl<T: Behavior> Index<BehaviorIndex> for BehaviorRefItem<'_, T> {
    type Output = T;

    fn index(&self, BehaviorIndex(index): BehaviorIndex) -> &Self::Output {
        if index == self.memory.stack.len() {
            self.current()
        } else {
            &self.memory[index]
        }
    }
}

/// Query data for a [`Behavior`] with mutable access.
///
/// # Usage
///
/// This provides a mutable reference to the current behavior state and all previous states in the stack.
///
/// See [`BehaviorRef`] for more details.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct BehaviorMut<T: Behavior> {
    current: Mut<'static, T>,
    memory: &'static mut Memory<T>,
    transition: &'static mut Transition<T>,
    sequence: Has<TransitionSequence<T>>,
}

impl<T: Behavior> BehaviorMutReadOnlyItem<'_, T> {
    /// See [`BehaviorRefItem::current`].
    pub fn current(&self) -> &T {
        &self.current
    }

    /// See [`BehaviorRefItem::previous`].
    pub fn previous(&self) -> Option<&T> {
        self.memory.last()
    }

    /// See [`BehaviorRefItem::iter`].
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.memory.iter().chain(std::iter::once(self.current()))
    }

    /// See [`BehaviorRefItem::index`].
    pub fn index(&self) -> usize {
        self.memory.len()
    }

    /// See [`BehaviorRefItem::has_transition`].
    pub fn has_transition(&self) -> bool {
        !self.transition.is_none()
    }

    /// See [`BehaviorRefItem::has_sequence`].
    pub fn has_sequence(&self) -> bool {
        self.sequence
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

impl<T: Behavior> DetectChanges for BehaviorMutReadOnlyItem<'_, T> {
    fn is_added(&self) -> bool {
        self.current.is_added()
    }

    fn is_changed(&self) -> bool {
        self.current.is_changed()
    }

    fn last_changed(&self) -> Tick {
        self.current.last_changed()
    }

    fn added(&self) -> Tick {
        self.current.added()
    }

    fn changed_by(&self) -> MaybeLocation {
        self.current.changed_by()
    }
}

impl<T: Behavior> Index<BehaviorIndex> for BehaviorMutReadOnlyItem<'_, T> {
    type Output = T;

    fn index(&self, BehaviorIndex(index): BehaviorIndex) -> &Self::Output {
        if index == self.memory.stack.len() {
            self.current()
        } else {
            &self.memory[index]
        }
    }
}

impl<T: Behavior> BehaviorMutItem<'_, T> {
    /// Returns the current [`Behavior`] state.
    pub fn current(&self) -> &T {
        &self.current
    }

    /// Returns the current [`Behavior`] state as a mutable.
    pub fn current_mut(&mut self) -> &mut T {
        self.current.as_mut()
    }

    /// See [`BehaviorRefItem::previous`].
    pub fn previous(&self) -> Option<&T> {
        self.memory.last()
    }

    /// Returns the previous [`Behavior`] state as a mutable.
    ///
    /// See [`BehaviorRefItem::previous`] for more details.
    pub fn previous_mut(&mut self) -> Option<&mut T> {
        self.memory.last_mut()
    }

    /// See [`BehaviorRefItem::iter`].
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.memory.iter().chain(std::iter::once(self.current()))
    }

    /// Returns a mutable iterator over all [`Behavior`] states in the stack, including the current one.
    ///
    /// See [`BehaviorRefItem::iter`] for more details.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
        self.memory
            .iter_mut()
            .chain(std::iter::once(self.current.as_mut()))
    }

    /// See [`BehaviorRefItem::index`].
    pub fn index(&self) -> usize {
        self.memory.len()
    }

    /// See [`BehaviorRefItem::has_transition`].
    pub fn has_transition(&self) -> bool {
        !self.transition.is_none()
    }

    /// See [`BehaviorRefItem::has_sequence`].
    pub fn has_sequence(&self) -> bool {
        self.sequence
    }

    /// Starts the given `next` behavior.
    ///
    /// This operation pushes the current behavior onto the stack and inserts the new behavior.
    ///
    /// Note that this will fail if the current behavior rejects `next` through [`Behavior::filter_next`].
    /// See [`Error`](crate::events::BehaviorEvent) for details on how to handle transition failures.
    #[track_caller]
    pub fn start(&mut self, next: T) {
        self.start_with_caller(next, MaybeLocation::caller());
    }

    /// Attempts to start the given `next` behavior if there are no pending transitions and the
    /// current behavior allows it.
    ///
    /// Note that it's still possible for the transition to fail if the current behavior mutates
    /// in such a way as to no longer allow the transition between the time this function is called
    /// and when the [`transition`](crate::transition::transition) system runs.
    ///
    /// Do **NOT** use this method to react to transition failures.
    /// See [`Error`](crate::events::BehaviorEvent) for details on how to correctly handle transition failures.
    ///
    /// # Usage
    ///
    /// This is similar to [`start`](BehaviorMutItem::start) but will return an error containing
    /// the given `next` behavior if it fails.
    ///
    /// This is useful for fire-and-forget transitions where you don't want to override a
    /// pending transition or may expect a transition failure.
    ///
    /// If multiple systems call this method before transition, only the first one will succeed.
    #[track_caller]
    pub fn try_start(&mut self, next: T) -> Result<(), T> {
        if self.has_transition() || !self.filter_next(&next) {
            return Err(next);
        }

        self.start_with_caller(next, MaybeLocation::caller());
        Ok(())
    }

    fn start_with_caller(&mut self, next: T, caller: MaybeLocation) {
        self.set_transition(Next(next), caller);
    }

    /// Interrupts the current behavior and starts the given `next` behavior.
    ///
    /// This operation stops all behaviors which yield to the new behavior and pushes it onto the stack.
    /// See [`Behavior::filter_yield`] for details on how to define how states yield to each other.
    ///
    /// This also removes any remaining [`TransitionSequence`] steps.
    ///
    /// The initial behavior is never allowed to yield.
    ///
    /// Note that this will fail if the first non-yielding behavior rejects `next` through [`Behavior::filter_next`].
    /// See [`Error`](crate::events::BehaviorEvent) for details on how to handle transition failures.
    #[track_caller]
    pub fn interrupt_start(&mut self, next: T) {
        self.interrupt_start_with_caller(next, MaybeLocation::caller());
    }

    #[track_caller]
    pub fn try_interrupt_start(&mut self, next: T) -> Result<(), T> {
        if self.has_transition() {
            return Err(next);
        }

        self.interrupt_start_with_caller(next, MaybeLocation::caller());
        Ok(())
    }

    fn interrupt_start_with_caller(&mut self, next: T, caller: MaybeLocation) {
        self.set_transition(Interrupt(next), caller);
    }

    /// Stops the current behavior.
    ///
    /// This operation pops the current behavior off the stack and resumes the previous behavior.
    ///
    /// Note that this will fail if the current behavior is the initial behavior.
    /// See [`Error`](crate::events::BehaviorEvent) for details on how to handle transition failures.
    #[track_caller]
    pub fn stop(&mut self) {
        self.stop_with_caller(MaybeLocation::caller());
    }

    #[track_caller]
    pub fn try_stop(&mut self) -> bool {
        if self.has_transition() || self.memory.is_empty() {
            return false;
        }

        self.stop_with_caller(MaybeLocation::caller());
        true
    }

    fn stop_with_caller(&mut self, caller: MaybeLocation) {
        self.set_transition(Previous, caller);
    }

    /// Stops the current and all previous behaviors and resumes the initial behavior.
    ///
    /// This operation clears the stack and resumes the initial behavior. It can never fail.
    /// If the stack is empty (i.e. initial behavior), it does nothing.
    #[track_caller]
    pub fn reset(&mut self) {
        self.reset_with_caller(MaybeLocation::caller());
    }

    #[track_caller]
    pub fn try_reset(&mut self) -> bool {
        if self.has_transition() {
            return false;
        }

        self.reset_with_caller(MaybeLocation::caller());
        true
    }

    fn reset_with_caller(&mut self, caller: MaybeLocation) {
        self.set_transition(Reset, caller);
    }

    fn set_transition(&mut self, transition: Transition<T>, caller: MaybeLocation) {
        let previous = replace(self.transition.as_mut(), transition);
        if !previous.is_none() {
            warn!(
                "transition override ({caller})): {previous:?} -> {:?}",
                *self.transition
            );
        }
    }

    fn push(
        &mut self,
        instance: Instance<T>,
        mut next: T,
        components: &Components,
        commands: &mut Commands,
    ) -> bool {
        let id = components.valid_component_id::<T>().unwrap();

        if self.filter_next(&next) {
            let previous = {
                swap(self.current.as_mut(), &mut next);
                next
            };
            self.invoke_start(Some(&previous), commands.instance(instance));
            let index = self.memory.len();
            if previous.is_resumable() {
                let next_index = index + 1;
                debug!(
                    "{instance:?}: {previous:?} (#{index}) -> {:?} (#{next_index})",
                    *self.current
                );

                previous.invoke_pause(&self.current, commands.instance(instance));

                commands.trigger_targets(
                    OnPause {
                        index: BehaviorIndex(index),
                    },
                    (*instance, id),
                );
                commands.trigger_targets(
                    OnStart {
                        index: BehaviorIndex(next_index),
                    },
                    (*instance, id),
                );

                self.memory.push(previous);
            } else {
                debug!(
                    "{instance:?}: {previous:?} (#{index}) -> {:?} (#{index})",
                    *self.current
                );

                previous.invoke_stop(&self.current, commands.instance(instance));

                commands.trigger_targets(OnStop { behavior: previous }, (*instance, id));
                commands.trigger_targets(
                    OnStart {
                        index: BehaviorIndex(index),
                    },
                    (*instance, id),
                );
            }
            true
        } else {
            warn!(
                "{instance:?}: transition {:?} -> {next:?} is not allowed",
                *self.current
            );

            commands.trigger_targets(
                OnError(TransitionError::RejectedNext(next)),
                (*instance, id),
            );
            false
        }
    }

    fn interrupt(
        &mut self,
        instance: Instance<T>,
        next: T,
        components: &Components,
        commands: &mut Commands,
    ) {
        let id = components.valid_component_id::<T>().unwrap();

        while self.filter_yield(&next) && !self.memory.is_empty() {
            let index = self.memory.len();
            if let Some(mut next) = self.memory.pop() {
                let next_index = self.memory.len();
                debug!(
                    "{instance:?}: {:?} (#{index}) -> {next:?} (#{next_index})",
                    *self.current
                );
                let previous = {
                    swap(self.current.as_mut(), &mut next);
                    next
                };
                previous.invoke_stop(&self.current, commands.instance(instance));
                commands.trigger_targets(OnStop { behavior: previous }, (*instance, id));
            }
        }

        self.push(instance, next, components, commands);
    }

    fn pop(
        &mut self,
        instance: Instance<T>,
        components: &Components,
        commands: &mut Commands,
    ) -> bool {
        let id = components.valid_component_id::<T>().unwrap();
        let index = self.memory.len();
        if let Some(mut next) = self.memory.pop() {
            let next_index = self.memory.len();
            debug!(
                "{instance:?}: {:?} (#{index}) -> {next:?} (#{next_index})",
                *self.current
            );
            let previous = {
                swap(self.current.as_mut(), &mut next);
                next
            };
            self.invoke_resume(&previous, commands.instance(instance));
            previous.invoke_stop(&self.current, commands.instance(instance));
            commands.trigger_targets(
                OnResume {
                    index: BehaviorIndex(next_index),
                },
                (*instance, id),
            );
            commands.trigger_targets(OnStop { behavior: previous }, (*instance, id));
            true
        } else {
            warn!(
                "{instance:?}: transition {:?} -> None is not allowed",
                *self.current
            );
            commands.trigger_targets(OnError::<T>(TransitionError::NoPrevious), (*instance, id));
            false
        }
    }

    fn clear(&mut self, instance: Instance<T>, components: &Components, commands: &mut Commands) {
        let id = components.valid_component_id::<T>().unwrap();
        while self.memory.len() > 1 {
            let index = self.memory.len();
            let previous = self.memory.pop().unwrap();
            let next_index = self.memory.len();
            let next = self.memory.last().unwrap();
            debug!(
                "{instance:?}: {:?} (#{index}) -> {next:?} (#{next_index})",
                *self.current
            );
            previous.invoke_stop(next, commands.instance(instance));
            commands.trigger_targets(OnStop { behavior: previous }, (*instance, id));
        }

        self.pop(instance, components, commands);
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

impl<T: Behavior> DetectChanges for BehaviorMutItem<'_, T> {
    fn is_added(&self) -> bool {
        self.current.is_added()
    }

    fn is_changed(&self) -> bool {
        self.current.is_changed()
    }

    fn last_changed(&self) -> Tick {
        self.current.last_changed()
    }

    fn added(&self) -> Tick {
        self.current.added()
    }

    fn changed_by(&self) -> MaybeLocation {
        self.current.changed_by()
    }
}

impl<T: Behavior> DetectChangesMut for BehaviorMutItem<'_, T> {
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

    fn set_added(&mut self) {
        self.current.set_added()
    }

    fn set_last_added(&mut self, last_added: Tick) {
        self.current.set_last_added(last_added)
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

impl<T: Behavior> Index<BehaviorIndex> for BehaviorMutItem<'_, T> {
    type Output = T;

    fn index(&self, BehaviorIndex(index): BehaviorIndex) -> &Self::Output {
        if index == self.memory.stack.len() {
            self.current()
        } else {
            &self.memory[index]
        }
    }
}

impl<T: Behavior> IndexMut<BehaviorIndex> for BehaviorMutItem<'_, T> {
    fn index_mut(&mut self, BehaviorIndex(index): BehaviorIndex) -> &mut Self::Output {
        if index == self.memory.stack.len() {
            self.current_mut()
        } else {
            &mut self.memory[index]
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Reflect)]
pub struct BehaviorIndex(usize);

impl BehaviorIndex {
    pub fn initial() -> Self {
        Self(0)
    }
}

#[derive(Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
struct Memory<T: Behavior> {
    stack: Vec<T>,
}

impl<T: Behavior> Default for Memory<T> {
    fn default() -> Self {
        Self {
            stack: Vec::default(),
        }
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
