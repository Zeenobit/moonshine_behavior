//! Each [`Behavior`] [`Transition`](crate::transition::Transition) triggers an [`Event`].
//!
//! You may use these events to react to transitions and define the behavior logic.
//! Most events contain a [`BehaviorIndex`] which may be used to query the current state of the behavior.
//!
//! See documentation for each event for more details.
//!
//! # Example
//! ```rust
//! use bevy::prelude::*;
//! use moonshine_behavior::prelude::*;
//!
//! #[derive(Component, Debug, Reflect)]
//! #[reflect(Component)]
//! struct B;
//!
//! impl Behavior for B {}
//!
//! fn on_start(trigger: Trigger<OnStart, B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(trigger.target()).unwrap();
//!     let state = &behavior[trigger.index];
//!     /* ... */
//! }
//!
//! fn on_pause(trigger: Trigger<OnPause, B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(trigger.target()).unwrap();
//!     let state = &behavior[trigger.index];
//!     /* ... */
//! }
//!
//! fn on_resume(trigger: Trigger<OnResume, B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(trigger.target()).unwrap();
//!     let state = &behavior[trigger.index];
//!     /* ... */
//! }
//!
//! fn on_activate(trigger: Trigger<OnActivate, B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(trigger.target()).unwrap();
//!     let state = &behavior[trigger.index];
//!     /* ... */
//! }
//!
//! fn on_stop(trigger: Trigger<OnStop<B>, B>) {
//!     let state = &trigger.behavior;
//!     /* ... */
//! }
//! ```

use bevy_ecs::prelude::*;

use crate::transition::TransitionError;
use crate::{Behavior, BehaviorIndex};

/// An event which is triggered when a [`Behavior`] starts.
#[derive(Event)]
pub struct OnStart {
    /// The index of the behavior that was started.
    pub index: BehaviorIndex,
    /// If true, it indicates the behavior stack is initializing.
    ///
    /// This parameter is always true for the initial behavior.
    /// You should check this flag to avoid performing redundant logic.
    ///
    /// For example, a `Bird` may perform a wing flap sound effect every time it starts flying.
    /// However, if you load a saved game (i.e. initialize) and the bird is already flying,
    /// you may want to skip the sound effect.
    pub initialize: bool,
}

/// An event which is triggered when a [`Behavior`] is paused.
#[derive(Event)]
pub struct OnPause {
    /// The index of the behavior that was paused.
    pub index: BehaviorIndex,
}

/// An event which is triggered when a [`Behavior`] is resumed.
#[derive(Event)]
pub struct OnResume {
    /// The index of the behavior that was resumed.
    pub index: BehaviorIndex,
}

/// An event which is triggered when a [`Behavior`] is started OR resumed.
///
#[derive(Event)]
pub struct OnActivate {
    /// The index of the behavior that was activated.
    pub index: BehaviorIndex,
    /// Whether the behavior was resumed or started.
    pub resume: bool,
    /// If true, it indicates the behavior stack is initializing.
    ///
    /// See [`OnStart::initialize`] for details.
    pub initialize: bool,
}

// TODO: OnSuspend, This gets tricky because the behavior state is already gone in `OnStop` ...

/// An event which is triggered when a [`Behavior`] is stopped.
///
/// Unlike other events, this one does not provide a behavior index.
#[derive(Event)]
pub struct OnStop<T: Behavior> {
    /// The index of the behavior that was stopped.
    pub index: BehaviorIndex,
    /// The behavior that was stopped.
    pub behavior: T,
    /// If true, it indicates the behavior was stopped because it was interrupted.
    pub interrupt: bool,
}

/// An event which is triggered when a [`TransitionError`] occurs.
#[derive(Event)]
pub struct OnError<T: Behavior>(pub TransitionError<T>);
