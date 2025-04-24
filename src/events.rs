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
        app.add_event::<BehaviorEvent<T>>();
    }
}

pub type BehaviorEvents<'w, 's, T> = EventReader<'w, 's, BehaviorEvent<T>>;
pub type BehaviorEventsMut<'w, T> = EventWriter<'w, BehaviorEvent<T>>;

/// An event sent during [`transition`](crate::transition::transition) to signal [`Behavior`] changes.
///
/// # Usage
///
/// Each successful transition results in either a [`Start`](BehaviorEvent::Start),
/// [`Pause`](BehaviorEvent::Pause), [`Resume`](BehaviorEvent::Resume), or [`Stop`](BehaviorEvent::Stop) event.
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
pub enum BehaviorEvent<T: Behavior> {
    /// Sent when a behavior is started.
    Start {
        /// The instance which is running the new behavior.
        instance: Instance<T>,
        /// The index of the new behavior.
        index: usize,
    },
    /// Sent when a behavior is paused.
    Pause {
        /// The instance which paused this behavior.
        instance: Instance<T>,
        /// The index of the paused behavior.
        index: usize,
    },
    /// Sent when a behavior is resumed.
    Resume {
        /// The instance which resumed this behavior.
        instance: Instance<T>,
        /// The index of the resumed behavior.
        index: usize,
    },
    Stop {
        /// The instance which stopped this behavior.
        instance: Instance<T>,
        /// The stopped behavior.
        behavior: T,
    },
    /// Sent when a behavior transition fails.
    Error {
        /// The instance which failed to transition.
        instance: Instance<T>,
        /// Reason for the failure.
        error: TransitionError<T>,
    },
}
