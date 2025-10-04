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
//! fn on_start(trigger: OnStart<B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(trigger.target()).unwrap();
//!     let state = &behavior[trigger.index];
//!     /* ... */
//! }
//!
//! fn on_pause(trigger: OnPause<B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(trigger.target()).unwrap();
//!     let state = &behavior[trigger.index];
//!     /* ... */
//! }
//!
//! fn on_resume(trigger: OnResume<B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(trigger.target()).unwrap();
//!     let state = &behavior[trigger.index];
//!     /* ... */
//! }
//!
//! fn on_activate(trigger: OnActivate<B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(trigger.target()).unwrap();
//!     let state = &behavior[trigger.index];
//!     /* ... */
//! }
//!
//! fn on_stop(trigger: OnStop<B>, B>) {
//!     let state = &trigger.behavior;
//!     /* ... */
//! }
//! ```

use bevy_ecs::event::EntityTrigger;
use bevy_ecs::prelude::*;
use moonshine_kind::{impl_entity_event_from_instance, prelude::*};

use crate::transition::TransitionError;
use crate::{Behavior, BehaviorIndex};

/// An event which is triggered when a [`Behavior`] starts.
///
/// See [`OnStart`] for a more ergonomic type alias for use in systems.
#[derive(Event)]
#[event(trigger = EntityTrigger)]
pub struct Start<T: Behavior> {
    /// Target of this [`EntityEvent`].
    pub instance: Instance<T>,
    /// The index of the behavior that was started.
    pub index: BehaviorIndex,
}

impl_entity_event_from_instance!(Start<T> where T: Behavior);

/// An event which is triggered when a [`Behavior`] is paused.
///
/// See [`OnPause`] for a more ergonomic type alias for use in systems.
#[derive(Event)]
#[event(trigger = EntityTrigger)]
pub struct Pause<T: Behavior> {
    /// Target of this [`EntityEvent`].
    pub instance: Instance<T>,
    /// The index of the behavior that was paused.
    pub index: BehaviorIndex,
}

impl_entity_event_from_instance!(Pause<T> where T: Behavior);

/// An event which is triggered when a [`Behavior`] is resumed.
///
/// See [`OnResume`] for a more ergonomic type alias for use in systems.
#[derive(Event)]
#[event(trigger = EntityTrigger)]
pub struct Resume<T: Behavior> {
    /// Target of this [`EntityEvent`].
    pub instance: Instance<T>,
    /// The index of the behavior that was resumed.
    pub index: BehaviorIndex,
}

impl_entity_event_from_instance!(Resume<T> where T: Behavior);

/// An event which is triggered when a [`Behavior`] is started OR resumed.
///
/// See [`OnActivate`] for a more ergonomic type alias for use in systems.
#[derive(Event)]
#[event(trigger = EntityTrigger)]
pub struct Activate<T: Behavior> {
    /// Target of this [`EntityEvent`].
    pub instance: Instance<T>,
    /// The index of the behavior that was activated.
    pub index: BehaviorIndex,
    /// Whether the behavior was resumed or started.
    pub resume: bool,
}

impl_entity_event_from_instance!(Activate<T> where T: Behavior);

// TODO: OnSuspend, This gets tricky because the behavior state is already gone in `OnStop` ...

/// An event which is triggered when a [`Behavior`] is stopped.
///
/// Unlike other events, this one does not provide a behavior index.
///
/// See [`OnStop`] for a more ergonomic type alias for use in systems.
#[derive(Event)]
#[event(trigger = EntityTrigger)]
pub struct Stop<T: Behavior> {
    /// Target of this [`EntityEvent`].
    pub instance: Instance<T>,
    /// The index of the behavior that was stopped.
    pub index: BehaviorIndex,
    /// The behavior that was stopped.
    pub behavior: T,
    /// If true, it indicates the behavior was stopped because it was interrupted.
    pub interrupt: bool,
}

impl_entity_event_from_instance!(Stop<T> where T: Behavior);

/// An event which is triggered when a [`TransitionError`] occurs.
///
/// See [`OnError`] for a more ergonomic type alias for use in systems.
#[derive(Event)]
#[event(trigger = EntityTrigger)]
pub struct Error<T: Behavior> {
    /// Target of this [`EntityEvent`].
    pub instance: Instance<T>,
    /// The associated [`TransitionError`].
    pub error: TransitionError<T>,
}

impl_entity_event_from_instance!(Error<T> where T: Behavior);

/// Alias for [`On<Start<T>>`](`Start`)
pub type OnStart<'w, 't, T> = On<'w, 't, Start<T>>;

/// Alias for [`On<Pause<T>>`](`Pause`)
pub type OnPause<'w, 't, T> = On<'w, 't, Pause<T>>;

/// Alias for [`On<Resume<T>>`](`Resume`)
pub type OnResume<'w, 't, T> = On<'w, 't, Resume<T>>;

/// Alias for [`On<Activate<T>>`](`Activate`)
pub type OnActivate<'w, 't, T> = On<'w, 't, Activate<T>>;

/// Alias for [`On<Stop<T>>`](`Stop`)
pub type OnStop<'w, 't, T> = On<'w, 't, Stop<T>>;

/// Alias for [`On<Error<T>>`](`Error`)
pub type OnError<'w, 't, T> = On<'w, 't, Error<T>>;
