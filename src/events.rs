//! Each [`Behavior`] [`Transition`](crate::transition::Transition) triggers an [`Event`].
//!
//! You may use these events to react to transitions and define the behavior logic.
//!
//! ## Behavior Indices
//!
//! Most events contain a [`BehaviorIndex`] which may be used to identify the exact state associated with the event.
//! You may use [`Index`](std::ops::Index) operator to access the associated state.
//!
//! ## Initialization
//!
//! On spawn, several [`Behavior`] states may be activated at the same time. This can happen if the entity is reloaded from
//! disk or synchronized from the network, for example.
//!
//! For technical reasons, you may need to perform different logic in such cases.
//!
//! For example, considering the following scenario:
//! Let's imagine we're trying to model a car. When the car starts, we want to play an engine start audio clip.
//! However, if the car has been turned on, and is now being loaded from disk, we should skip the engine start audio clip.
//! You may use the `.initial` flag on the behavior events to branch your logic.
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
//! fn on_start(event: OnStart<B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(*event.instance).unwrap();
//!     let state = &behavior[event.index];
//!
//!     if event.initial {
//!         /* Custom initialization logic */
//!     }
//!
//!     /* ... */
//! }
//!
//! fn on_pause(event: OnPause<B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(*event.instance).unwrap();
//!     let state = &behavior[event.index];
//!     /* ... */
//! }
//!
//! fn on_resume(event: OnResume<B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(*event.instance).unwrap();
//!     let state = &behavior[event.index];
//!     /* ... */
//! }
//!
//! fn on_activate(event: OnActivate<B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(*event.instance).unwrap();
//!     let state = &behavior[event.index];
//!     /* ... */
//! }
//!
//! fn on_stop(event: OnStop<B>) {
//!     let state = &event.behavior;
//!     /* ... */
//! }
//! ```
//!
//! ##

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
    /// If true, it implies the behavior is initializing. See [module documention](self) for details.
    pub initial: bool,
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
    /// If true, it implies the behavior is initializing. See [module documention](self) for details.
    pub initial: bool,
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
