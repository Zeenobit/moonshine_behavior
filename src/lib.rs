#![doc = include_str!("../README.md")]

pub mod prelude {
    pub use crate::{
        {transition, InvalidTransition, Transition, TransitionResult}, {Behavior, BehaviorPlugin},
        {Paused, Previous, Resumed, Started, Stopped},
        {PausedEvent, ResumedEvent, StartedEvent, StoppedEvent},
    };
}

mod events;
mod memory;
mod transition;

use std::{fmt::Debug, marker::PhantomData};

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_reflect::{FromReflect, GetTypeRegistration, Typed};
use moonshine_util::future::Future;

pub use events::*;
pub use memory::*;
pub use transition::*;

pub struct BehaviorPlugin<B> {
    pub send_events: bool,
    pub marker: PhantomData<B>,
}

impl<B> Default for BehaviorPlugin<B> {
    fn default() -> Self {
        Self {
            send_events: true,
            marker: PhantomData,
        }
    }
}

impl<B: RegisterableBehavior> Plugin for BehaviorPlugin<B> {
    fn build(&self, app: &mut App) {
        app.register_type::<Memory<B>>()
            .register_type::<Transition<B>>();

        if self.send_events {
            #[allow(deprecated)]
            behavior_events_plugin::<B>(app);
        }
    }
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
#[deprecated(since = "0.1.6", note = "use `BehaviorPlugin` instead")]
pub fn behavior_plugin<B: RegisterableBehavior>(app: &mut App) {
    app.add_plugins(BehaviorPlugin::<B>::default());
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
#[deprecated(since = "0.1.6", note = "use `BehaviorPlugin` instead")]
pub fn behavior_events_plugin<B: Behavior>(app: &mut App) {
    app.add_event::<StartedEvent<B>>()
        .add_event::<PausedEvent<B>>()
        .add_event::<ResumedEvent<B>>()
        .add_event::<StoppedEvent<B>>();
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
///         match self {
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
/// #[derive(Component,  Debug, Reflect)]
/// #[reflect(Component)]
/// enum Bird {
///     Idle(Wait),
///     Fly(WingState),
///     Sleep(Wait),
///     Chirp,
/// }
///
/// impl Default for Bird {
///    fn default() -> Self {
///       Bird::Idle(Wait::default())
///    }
/// }
///
/// #[derive(Default, Debug, Reflect)]
/// struct Wait(Duration);
///
/// #[derive(Default, Debug, Reflect)]
/// enum WingState {
///     #[default]
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
pub trait RegisterableBehavior: Behavior + FromReflect + GetTypeRegistration + Typed {}

impl<B: Behavior + FromReflect + GetTypeRegistration + Typed> RegisterableBehavior for B {}

/// A [`Bundle`] which contains a [`Behavior`] and other required components for it to function.
#[derive(Bundle, Default)]
#[deprecated(since = "0.1.6", note = "use `#[require(Transition::<B>)]` instead")]
pub struct BehaviorBundle<B: Behavior> {
    pub behavior: B,
    pub memory: Memory<B>,
    pub transition: Transition<B>,
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
            transition: Transition::default(),
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
