use bevy_ecs::{prelude::*, system::SystemParam};

use moonshine_kind::prelude::*;

use crate::Behavior;

#[derive(SystemParam)]
pub struct BehaviorEvents<'w, 's, T: Behavior> {
    start: EventReader<'w, 's, Start<T>>,
    pause: EventReader<'w, 's, Pause<T>>,
    resume: EventReader<'w, 's, Resume<T>>,
    stop: EventReader<'w, 's, Stop<T>>,
}

impl<T: Behavior> BehaviorEvents<'_, '_, T> {
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
}

#[derive(SystemParam)]
pub struct BehaviorEventsMut<'w, T: Behavior> {
    start: EventWriter<'w, Start<T>>,
    pause: EventWriter<'w, Pause<T>>,
    resume: EventWriter<'w, Resume<T>>,
    stop: EventWriter<'w, Stop<T>>,
}

impl<T: Behavior> BehaviorEventsMut<'_, T> {
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
}

pub type StartEvents<'w, 's, T> = EventReader<'w, 's, Start<T>>;
pub type PauseEvents<'w, 's, T> = EventReader<'w, 's, Pause<T>>;
pub type ResumeEvents<'w, 's, T> = EventReader<'w, 's, Resume<T>>;
pub type StopEvents<'w, 's, T> = EventReader<'w, 's, Stop<T>>;

#[derive(Event)]
pub struct Start<T: Behavior> {
    pub instance: Instance<T>,
}

#[derive(Event)]
pub struct Pause<T: Behavior> {
    pub instance: Instance<T>,
}

#[derive(Event)]
pub struct Resume<T: Behavior> {
    pub instance: Instance<T>,
}

#[derive(Event)]
pub struct Stop<T: Behavior> {
    pub instance: Instance<T>,
    pub behavior: T,
}
