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

use bevy_ecs::event::EntityComponentsTrigger;
use bevy_ecs::prelude::*;

use crate::transition::TransitionError;
use crate::{Behavior, BehaviorIndex};

/// An event which is triggered when a [`Behavior`] starts.
#[derive(EntityEvent)]
#[entity_event(trigger = EntityComponentsTrigger<'a>)]
pub struct Start {
    /// Target of this [`EntityEvent`].
    pub entity: Entity,
    /// The index of the behavior that was started.
    pub index: BehaviorIndex,
}

/// An event which is triggered when a [`Behavior`] is paused.
#[derive(EntityEvent)]
#[entity_event(trigger = EntityComponentsTrigger<'a>)]
pub struct Pause {
    /// Target of this [`EntityEvent`].
    pub entity: Entity,
    /// The index of the behavior that was paused.
    pub index: BehaviorIndex,
}

/// An event which is triggered when a [`Behavior`] is resumed.
#[derive(EntityEvent)]
#[entity_event(trigger = EntityComponentsTrigger<'a>)]
pub struct Resume {
    /// Target of this [`EntityEvent`].
    pub entity: Entity,
    /// The index of the behavior that was resumed.
    pub index: BehaviorIndex,
}

/// An event which is triggered when a [`Behavior`] is started OR resumed.
///
#[derive(EntityEvent)]
#[entity_event(trigger = EntityComponentsTrigger<'a>)]
pub struct Activate {
    /// Target of this [`EntityEvent`].
    pub entity: Entity,
    /// The index of the behavior that was activated.
    pub index: BehaviorIndex,
    /// Whether the behavior was resumed or started.
    pub resume: bool,
}

// TODO: OnSuspend, This gets tricky because the behavior state is already gone in `OnStop` ...

/// An event which is triggered when a [`Behavior`] is stopped.
///
/// Unlike other events, this one does not provide a behavior index.
#[derive(EntityEvent)]
#[entity_event(trigger = EntityComponentsTrigger<'a>)]
pub struct Stop<T: Behavior> {
    /// Target of this [`EntityEvent`].
    pub entity: Entity,
    /// The index of the behavior that was stopped.
    pub index: BehaviorIndex,
    /// The behavior that was stopped.
    pub behavior: T,
    /// If true, it indicates the behavior was stopped because it was interrupted.
    pub interrupt: bool,
}

/// An event which is triggered when a [`TransitionError`] occurs.
#[derive(EntityEvent)]
#[entity_event(trigger = EntityComponentsTrigger<'a>)]
pub struct Error<T: Behavior> {
    /// Target of this [`EntityEvent`].
    pub entity: Entity,
    /// The associated [`TransitionError`].
    pub error: TransitionError<T>,
}
