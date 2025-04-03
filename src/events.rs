use std::marker::PhantomData;

use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, system::SystemParam};

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
        app.add_event::<Start<T>>()
            .add_event::<Pause<T>>()
            .add_event::<Resume<T>>()
            .add_event::<Stop<T>>()
            .add_event::<Error<T>>();
    }
}

#[derive(SystemParam)]
pub struct BehaviorEvents<'w, 's, T: Behavior + Component> {
    start: EventReader<'w, 's, Start<T>>,
    pause: EventReader<'w, 's, Pause<T>>,
    resume: EventReader<'w, 's, Resume<T>>,
    stop: EventReader<'w, 's, Stop<T>>,
    error: EventReader<'w, 's, Error<T>>,
}

impl<T: Behavior + Component> BehaviorEvents<'_, '_, T> {
    pub fn start(&mut self) -> impl Iterator<Item = Instance<T>> + '_ {
        self.start.read().map(|&Start { instance }| instance)
    }

    pub fn resume(&mut self) -> impl Iterator<Item = Instance<T>> + '_ {
        self.resume.read().map(|&Resume { instance }| instance)
    }

    pub fn pause(&mut self) -> impl Iterator<Item = Instance<T>> + '_ {
        self.pause.read().map(|&Pause { instance }| instance)
    }

    pub fn stop(&mut self) -> impl Iterator<Item = (Instance<T>, &T)> + '_ {
        self.stop
            .read()
            .map(|Stop { instance, behavior }| (*instance, behavior))
    }

    pub fn error(&mut self) -> impl Iterator<Item = (Instance<T>, &TransitionError<T>)> + '_ {
        self.error
            .read()
            .map(|Error { instance, error }| (*instance, error))
    }
}

#[derive(SystemParam)]
pub struct BehaviorEventsMut<'w, T: Behavior + Component> {
    start: EventWriter<'w, Start<T>>,
    pause: EventWriter<'w, Pause<T>>,
    resume: EventWriter<'w, Resume<T>>,
    stop: EventWriter<'w, Stop<T>>,
    error: EventWriter<'w, Error<T>>,
}

impl<T: Behavior + Component> BehaviorEventsMut<'_, T> {
    pub(crate) fn start(&mut self, instance: Instance<T>) {
        self.start.send(Start { instance });
    }

    pub(crate) fn resume(&mut self, instance: Instance<T>) {
        self.resume.send(Resume { instance });
    }

    pub(crate) fn pause(&mut self, instance: Instance<T>) {
        self.pause.send(Pause { instance });
    }

    pub(crate) fn stop(&mut self, instance: Instance<T>, behavior: T) {
        self.stop.send(Stop { instance, behavior });
    }

    pub(crate) fn error(&mut self, instance: Instance<T>, error: TransitionError<T>) {
        self.error.send(Error { instance, error });
    }
}

#[derive(Event)]
struct Start<T: Behavior + Component> {
    pub instance: Instance<T>,
}

#[derive(Event)]
struct Pause<T: Behavior + Component> {
    pub instance: Instance<T>,
}

#[derive(Event)]
struct Resume<T: Behavior + Component> {
    pub instance: Instance<T>,
}

#[derive(Event)]
struct Stop<T: Behavior + Component> {
    pub instance: Instance<T>,
    pub behavior: T,
}

#[derive(Event)]
struct Error<T: Behavior + Component> {
    pub instance: Instance<T>,
    pub error: TransitionError<T>,
}
