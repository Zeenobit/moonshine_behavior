#![allow(deprecated)] // TODO: Remove after 0.2.0
#![doc = include_str!("../README.md")]

use std::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use bevy_app::{App, Plugin};
use bevy_ecs::{component::Tick, prelude::*, query::QueryData};
use bevy_reflect::{FromReflect, GetTypeRegistration, TypePath};
use moonshine_util::future::Future;

pub mod prelude {
    pub use crate::{
        behavior_plugin, {transition, InvalidTransition, Transition, TransitionResult},
        {Behavior, BehaviorBundle}, {Paused, Resumed, Started, Stopped},
        {PausedEvent, ResumedEvent, StartedEvent, StoppedEvent},
    };

    // TODO: To be removed after 0.2.0
    pub use crate::{BehaviorMut, BehaviorRef};
}

/// Returns a [`Plugin`] which registers the given [`Behavior`] with the [`App`].
///
/// # Usage
/// Add this plugin to your application during initialization for every behavior.
///
/// This plugin is required for behavior transition events to be sent at runtime.
/// Without it, the [`transition`] system will [`panic`].
///
/// This plugin requires the behavior to support reflection (see [`Reflect`]).
/// If `B` does not support reflection, use [`behavior_events_plugin`] instead.
///
/// [`Reflect`]: bevy_reflect::Reflect
/// [`transition`]: crate::transition::transition
pub fn behavior_plugin<B: RegisterableBehavior>() -> impl Plugin {
    |app: &mut App| {
        app.add_plugins(behavior_events_plugin::<B>())
            .register_type::<Memory<B>>()
            .register_type::<Transition<B>>();
    }
}

/// Returns a [`Plugin`] which adds the [`Behavior`] events to the [`App`].
///
/// # Usage
/// Add this plugin to your application during initialization for every behavior which does not support reflection.
///
/// This plugin is required for behavior transition events to be sent at runtime.
/// Without it, the [`transition`] system will [`panic`].
///
/// For behaviors which do support reflection, prefer to use [`behavior_plugin`] instead.
///
/// [`transition`]: crate::transition::transition
pub fn behavior_events_plugin<B: Behavior>() -> impl Plugin {
    |app: &mut App| {
        app.add_event::<StartedEvent<B>>()
            .add_event::<PausedEvent<B>>()
            .add_event::<ResumedEvent<B>>()
            .add_event::<StoppedEvent<B>>();
    }
}

/// A [`Component`] which represents some state of its [`Entity`].
///
/// # Example
/// Typically, a behavior is implemented as an `enum` type, which acts like a single state within a state machine:
/// ```
/// use bevy::prelude::*;
/// use moonshine_behavior::prelude::*;
///
/// #[derive(Component, Default, Debug, Reflect)]
/// #[reflect(Component)]
/// enum Bird {
///     #[default]
///     Idle,
///     Fly,
///     Sleep,
///     Chirp
/// }
///
/// impl Behavior for Bird {
///     fn allows_next(&self, next: &Self) -> bool {
///         use Bird::*;
///         match (self, next) {
///             // Bird may Fly or Sleep when Idle:
///             Idle => matches!(next, Fly | Sleep),
///             // Bird may Chirp when Flying:
///             Fly => matches!(next, Chirp),
///             // Bird must not do anything else if Sleeping or Chirping:
///             _ => false,
///         }
///     }
/// }
/// ```
/// However, a behavior can also be any struct type:
/// ```
/// # use bevy::prelude::*;
/// # use moonshine_behavior::prelude::*;
///
/// #[derive(Component, Default, Debug, Reflect)]
/// #[reflect(Component)]
/// struct Bird {
///     fly: bool,
///     sleep: bool,
///     chirp: bool,
/// }
///
/// impl Behavior for Bird {
///     fn allows_next(&self, next: &Self) -> bool {
///         if self.sleep || self.chirp {
///             // Bird must not do anything else if Sleeping or Chirping:
///             false
///         } else if self.fly {
///             // Bird may Chirp when Flying:
///             next.chirp
///         } else {
///             // Bird may Fly or Sleep when Idle:
///             next.fly || next.sleep
///         }
///     }
/// }
/// ```
/// You can also combine nested enums and structs to create complex state machines:
/// ```
/// use std::time::Duration;
///
/// use bevy::prelude::*;
/// use moonshine_behavior::prelude::*;
///
/// #[derive(Component, Default, Debug, Reflect)]
/// #[reflect(Component)]
/// enum Bird {
///     #[default]
///     Idle(Wait),
///     Fly(WingState),
///     Sleep(Wait),
///     Chirp,
/// }
///
/// #[derive(Default, Debug, Reflect)]
/// struct Wait(Duration);
///
/// #[derive(Default, Debug, Reflect)]
/// enum WingState {
///     Up,
///     Down,
/// }
///
/// impl Behavior for Bird {
///     /* ... */
/// }
/// ```
pub trait Behavior: Component + Debug + Sized {
    /// Returns `true` if some next [`Behavior`] is allowed to be started after this one.
    ///
    /// By default, any behavior is allowed to start after any other behavior.
    fn allows_next(&self, _next: &Self) -> bool {
        true
    }

    /// Returns `true` if this [`Behavior`] may be resumed after it has been paused.
    ///
    /// By default, all behaviors are resumable.
    fn is_resumable(&self) -> bool {
        true
    }

    /// This method is called when the current [`Behavior`] is started.
    ///
    /// By default, it does nothing.
    /// Optionally, it may return the next [`Behavior`] to start immediately after this one.
    ///
    /// This allows you to create a chain of behaviors that execute in sequence.
    fn started(&self) -> Option<Self> {
        None
    }

    /// This method is called when the current [`Behavior`] is stopped.
    ///
    /// By default, it does nothing.
    /// Optionally, it may return the next [`Behavior`] to start immediately after this one.
    ///
    /// This allows you to create a chain of behaviors that execute in sequence.
    fn stopped(&self) -> Option<Self> {
        None
    }
}

#[doc(hidden)]
pub trait RegisterableBehavior: Behavior + FromReflect + TypePath + GetTypeRegistration {}

impl<B: Behavior + FromReflect + TypePath + GetTypeRegistration> RegisterableBehavior for B {}

/// A [`Bundle`] which contains a [`Behavior`] and other required components for it to function.
#[derive(Bundle, Default)]
pub struct BehaviorBundle<B: Behavior> {
    behavior: B,
    memory: Memory<B>,
    transition: Transition<B>,
}

impl<B: Behavior + Clone> Clone for BehaviorBundle<B> {
    fn clone(&self) -> Self {
        Self {
            behavior: self.behavior.clone(),
            memory: self.memory.clone(),
            transition: self.transition.clone(),
        }
    }
}

impl<B: Behavior> BehaviorBundle<B> {
    pub fn new(behavior: B) -> Self {
        assert!(
            behavior.is_resumable(),
            "initial behavior must be resumable"
        );
        Self {
            behavior,
            memory: Memory::default(),
            transition: Transition::Started, // Initial Behavior
        }
    }

    /// Tries to start the given [`Behavior`] as the next one immediately after insertion.
    pub fn try_start(&mut self, next: B) -> Future<TransitionResult<B>> {
        let (transition, future) = Transition::next(next);
        self.transition = transition;
        future
    }
}

impl<B: Behavior> From<B> for BehaviorBundle<B> {
    fn from(behavior: B) -> Self {
        Self::new(behavior)
    }
}

/// A [`QueryData`] used to query a [`Behavior`].
#[derive(QueryData)]
#[deprecated(since = "0.1.5", note = "query behavior components directly")]
pub struct BehaviorRef<B: Behavior> {
    behavior: Ref<'static, B>,
    memory: &'static Memory<B>,
    transition: &'static Transition<B>,
}

impl<B: Behavior> BehaviorRefItem<'_, B> {
    /// Returns a reference to the current [`Behavior`].
    #[deprecated(since = "0.1.5", note = "query behavior component directly")]
    pub fn get(&self) -> &B {
        &self.behavior
    }

    /// Returns `true` if the current [`Behavior`] was just started.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_started` instead")]
    pub fn is_started(&self) -> bool {
        self.transition.is_started() || self.is_added()
    }

    /// Returns `true` if the current [`Behavior`] was just resumed.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_resumed` instead")]
    pub fn is_resumed(&self) -> bool {
        self.transition.is_resumed()
    }

    /// Returns `true` if the current [`Behavior`] is active and not in a transition.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_stable` instead")]
    pub fn is_stable(&self) -> bool {
        self.transition.is_stable()
    }

    #[deprecated(since = "0.1.5", note = "query behavior component directly")]
    pub fn get_changed(&self) -> Option<&B> {
        self.is_changed().then(|| self.get())
    }

    /// Returns `true` if a [`Transition`] is currently in progress.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_suspending` instead")]
    pub fn has_transition(&self) -> bool {
        self.transition.is_suspending()
    }

    /// Returns a reference to the previous [`Behavior`], if it exists.
    #[deprecated(since = "0.1.5", note = "use `Memory::<B>::previous` instead")]
    pub fn previous(&self) -> Option<&B> {
        self.memory.previous()
    }

    /// Returns a reference to the [`Behavior`] [`Memory`].
    #[deprecated(since = "0.1.5", note = "query `Memory<B>` directly")]
    pub fn memory(&self) -> &Memory<B> {
        self.memory
    }
}

impl<B: Behavior> Deref for BehaviorRefItem<'_, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.behavior
    }
}

impl<B: Behavior> Borrow<B> for BehaviorRefItem<'_, B> {
    fn borrow(&self) -> &B {
        &self.behavior
    }
}

impl<B: Behavior> DetectChanges for BehaviorRefItem<'_, B> {
    fn is_added(&self) -> bool {
        self.behavior.is_added()
    }

    fn is_changed(&self) -> bool {
        self.behavior.is_changed()
    }

    fn last_changed(&self) -> Tick {
        self.behavior.last_changed()
    }
}

/// A mutable [`QueryData`] used to query and manipulate a [`Behavior`].
#[derive(QueryData)]
#[query_data(mutable)]
#[deprecated(
    since = "0.1.5",
    note = "query behavior components directly; use `&mut Transition::<B>` to invoke transitions"
)]
pub struct BehaviorMut<B: Behavior> {
    behavior: Mut<'static, B>,
    memory: &'static Memory<B>,
    transition: &'static mut Transition<B>,
}

impl<B: Behavior> BehaviorMutReadOnlyItem<'_, B> {
    /// Returns a reference to the current [`Behavior`].
    #[deprecated(since = "0.1.5", note = "query behavior component directly")]
    pub fn get(&self) -> &B {
        &self.behavior
    }

    /// Returns `true` if the current [`Behavior`] was just started.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_started` instead")]
    pub fn is_started(&self) -> bool {
        self.transition.is_started() || self.is_added()
    }

    /// Returns `true` if the current [`Behavior`] was just resumed.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_resumed` instead")]
    pub fn is_resumed(&self) -> bool {
        self.transition.is_resumed()
    }

    /// Returns `true` if the current [`Behavior`] is active and not in a transition.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_stable` instead")]
    pub fn is_stable(&self) -> bool {
        self.transition.is_stable()
    }

    /// Returns a reference to the current [`Behavior`] if it was changed since last query.
    #[deprecated(since = "0.1.5", note = "query behavior component directly")]
    pub fn get_changed(&self) -> Option<&B> {
        self.is_changed().then(|| self.get())
    }

    /// Returns `true` if a [`Transition`] is currently in progress.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_suspending` instead")]
    pub fn has_transition(&self) -> bool {
        self.transition.is_suspending()
    }

    /// Returns a reference to the previous [`Behavior`], if it exists.
    #[deprecated(since = "0.1.5", note = "use `Memory::<B>::previous` instead")]
    pub fn previous(&self) -> Option<&B> {
        self.memory.previous()
    }

    /// Returns a reference to the [`Behavior`] [`Memory`].
    #[deprecated(since = "0.1.5", note = "query `Memory<B>` directly")]
    pub fn memory(&self) -> &Memory<B> {
        self.memory
    }
}

impl<B: Behavior> Deref for BehaviorMutReadOnlyItem<'_, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.behavior
    }
}

impl<B: Behavior> Borrow<B> for BehaviorMutReadOnlyItem<'_, B> {
    fn borrow(&self) -> &B {
        &self.behavior
    }
}

impl<B: Behavior> DetectChanges for BehaviorMutReadOnlyItem<'_, B> {
    fn is_added(&self) -> bool {
        self.behavior.is_added()
    }

    fn is_changed(&self) -> bool {
        self.behavior.is_changed()
    }

    fn last_changed(&self) -> Tick {
        self.behavior.last_changed()
    }
}

impl<B: Behavior> BehaviorMutItem<'_, B> {
    /// Returns a reference to the current [`Behavior`].
    #[deprecated(since = "0.1.5", note = "query behavior component directly")]
    pub fn get(&self) -> &B {
        &self.behavior
    }

    /// Returns a mutable reference to the current [`Behavior`].
    #[deprecated(since = "0.1.5", note = "query behavior component directly")]
    pub fn get_mut(&mut self) -> &mut B {
        &mut self.behavior
    }

    /// Returns `true` if the current [`Behavior`] was just started.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_started` instead")]
    pub fn is_started(&self) -> bool {
        self.transition.is_started() || self.is_added()
    }

    /// Returns `true` if the current [`Behavior`] was just resumed.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_resumed` instead")]
    pub fn is_resumed(&self) -> bool {
        self.transition.is_resumed()
    }

    /// Returns `true` if the current [`Behavior`] is active and not in a transition.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_stable` instead")]
    pub fn is_stable(&self) -> bool {
        self.transition.is_stable()
    }

    /// Returns a reference to the current [`Behavior`] if it was changed since last query.
    #[deprecated(since = "0.1.5", note = "query behavior component directly")]
    pub fn get_changed(&self) -> Option<&B> {
        self.is_changed().then(|| self.get())
    }

    /// Returns a mutable reference to the current [`Behavior`] if it was changed since last query.
    #[deprecated(since = "0.1.5", note = "query behavior component directly")]
    pub fn get_changed_mut(&mut self) -> Option<&mut B> {
        self.is_changed().then(|| self.get_mut())
    }

    /// Returns `true` if a [`Transition`] is currently in progress.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::is_suspending` instead")]
    pub fn has_transition(&self) -> bool {
        self.transition.is_suspending()
    }

    /// Tries to start the given [`Behavior`] as the next one.
    ///
    /// This only sets the behavior [`Transition`], and does not immediately modify the behavior.
    /// The next behavior will only start if it is allowed to by the [`transition()`] system.
    /// Otherwise, the transition is ignored.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::try_start` instead")]
    pub fn try_start(&mut self, next: B) -> Future<TransitionResult<B>> {
        self.transition.try_start(next)
    }

    /// Stops the current [`Behavior`] and tries to resume the previous one.
    ///
    /// If the previous behavior is not resumable, the behavior before it is tried,
    /// and so on until a resumable behavior is found.
    ///
    /// If the current behavior is the initial one, it does nothing.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::stop` instead")]
    pub fn stop(&mut self) {
        self.transition.stop();
    }

    /// Stops the current [`Behavior`] and resumes the initial one.
    ///
    /// If the current behavior is the initial one, it does nothing.
    #[deprecated(since = "0.1.5", note = "use `Transition::<B>::reset` instead")]
    pub fn reset(&mut self) {
        self.transition.reset();
    }

    /// Returns a reference to the previous [`Behavior`], if it exists.
    #[deprecated(since = "0.1.5", note = "use `Memory::<B>::previous` instead")]
    pub fn previous(&self) -> Option<&B> {
        self.memory.previous()
    }

    /// Returns a reference to the [`Behavior`] [`Memory`].
    #[deprecated(since = "0.1.5", note = "query `Memory<B>` directly")]
    pub fn memory(&self) -> &Memory<B> {
        self.memory
    }
}

impl<B: Behavior> Deref for BehaviorMutItem<'_, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        self.behavior.as_ref()
    }
}

impl<B: Behavior> DerefMut for BehaviorMutItem<'_, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.behavior.as_mut()
    }
}

impl<B: Behavior> Borrow<B> for BehaviorMutItem<'_, B> {
    fn borrow(&self) -> &B {
        self.behavior.as_ref()
    }
}

impl<B: Behavior> BorrowMut<B> for BehaviorMutItem<'_, B> {
    fn borrow_mut(&mut self) -> &mut B {
        self.behavior.as_mut()
    }
}

impl<B: Behavior> DetectChanges for BehaviorMutItem<'_, B> {
    fn is_added(&self) -> bool {
        self.behavior.is_added()
    }

    fn is_changed(&self) -> bool {
        self.behavior.is_changed()
    }

    fn last_changed(&self) -> Tick {
        self.behavior.last_changed()
    }
}

impl<B: Behavior> DetectChangesMut for BehaviorMutItem<'_, B> {
    type Inner = B;

    fn set_changed(&mut self) {
        self.behavior.set_changed();
    }

    fn set_last_changed(&mut self, last_changed: Tick) {
        self.behavior.set_last_changed(last_changed);
    }

    fn bypass_change_detection(&mut self) -> &mut Self::Inner {
        self.behavior.bypass_change_detection()
    }
}

mod events;
mod memory;
mod transition;

pub use events::*;
pub use memory::*;
pub use transition::*;
