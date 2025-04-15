use std::marker::PhantomData;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use moonshine_kind::prelude::*;

use crate::transition::TransitionError;
use crate::Behavior;

pub struct BehaviorEventsPlugin<T: Behavior + Component>(PhantomData<T>);

impl<T: Behavior + Component> Default for BehaviorEventsPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Behavior + Component> Plugin for BehaviorEventsPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_event::<TransitionEvent<T>>();
    }
}

pub type TransitionEvents<'w, 's, T> = EventReader<'w, 's, TransitionEvent<T>>;
pub type TransitionEventsMut<'w, T> = EventWriter<'w, TransitionEvent<T>>;

#[derive(Event, Debug, PartialEq)]
pub enum TransitionEvent<T: Behavior + Component> {
    Start {
        instance: Instance<T>,
        index: usize,
    },
    Pause {
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
    Error {
        instance: Instance<T>,
        error: TransitionError<T>,
    },
}
