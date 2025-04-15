use std::marker::PhantomData;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use moonshine_kind::prelude::*;

use crate::transition::TransitionError;
use crate::Behavior;

pub struct BehaviorEventsPlugin<T: Behavior>(PhantomData<T>);

impl<T: Behavior> Default for BehaviorEventsPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Behavior> Plugin for BehaviorEventsPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_event::<TransitionEvent<T>>();
    }
}

pub type TransitionEvents<'w, 's, T> = EventReader<'w, 's, TransitionEvent<T>>;
pub type TransitionEventsMut<'w, T> = EventWriter<'w, TransitionEvent<T>>;

/// An event sent during [`transition`](crate::transition::transition) to signal [`Behavior`] changes.
///
/// # Usage
///
/// Each successful transition results in either a [`Start`](TransitionEvent::Start),
/// [`Pause`](TransitionEvent::Pause), [`Resume`](TransitionEvent::Resume), or [`Stop`](TransitionEvent::Stop) event.
///
/// Most events carry the [`Instance`](crate::Instance) and index of the associated behavior.
///
/// This index maybe used with a [`BehaviorRef`](crate::BehaviorRefItem) or [`BehaviorMut`](crate::BehaviorMutItem)
/// to access a specific instance of the behavior.
///
/// The index can be used to distinguish between different states if:
/// - More than one variation of the same state exists in the stack, or
/// - Multiple transitions have ocurred since the last query.
#[derive(Event, Debug, PartialEq)]
pub enum TransitionEvent<T: Behavior> {
    /// Sent when a behavior is started.
    Start {
        /// The instance which is running the new behavior.
        instance: Instance<T>,
        /// The index of the new behavior.
        ///
        /// Typically, for start events, this always matches the [current behavior index](crate::BehaviorRefItem::index).
        /// The exception is if multiple behaviors were started on the same instance since the last query.
        /// To access the behavior by index, use the [`index`](std::ops::Index::index) operator on a [`BehaviorRef`](crate::BehaviorRefItem).
        index: usize,
    },
    /// Sent when a behavior is paused.
    Pause {
        /// The instance which paused this behavior.
        instance: Instance<T>,
        index: usize,
    },
    Resume {
        instance: Instance<T>,
        index: usize,
    },
    Stop {
        instance: Instance<T>,
        behavior: T,
    },
    /// Sent when a behavior transition fails.
    Error {
        instance: Instance<T>,
        error: TransitionError<T>,
    },
}
