//! Each [`Behavior`] [`Transition`](crate::transition::Transition) triggers an [`Event`].
//!
//! You may use these events to react to transitions and define the behavior logic.
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
//! fn b_start(trigger: Trigger<OnStart, B>, query: Query<BehaviorRef<B>>) {
//!     let behavior = query.get(target.target()).unwrap();
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
#[derive(Event)]
pub struct OnActivate {
    /// The index of the behavior that was activated.
    pub index: BehaviorIndex,
    /// Whether the behavior was resumed or started.
    pub resume: bool,
}

/// An event which is triggered when a [`Behavior`] is stopped.
#[derive(Event)]
pub struct OnStop<T: Behavior> {
    /// The index of the behavior that was stopped.
    pub behavior: T,
}

/// An event which is triggered when a [`TransitionError`] occurs.
#[derive(Event)]
pub struct OnError<T: Behavior>(pub TransitionError<T>);
