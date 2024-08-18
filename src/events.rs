use bevy_ecs::{prelude::*, system::SystemParam};

use moonshine_kind::prelude::*;

use crate::Behavior;

#[doc(hidden)]
#[derive(SystemParam)]
pub struct BehaviorEventWriter<'w, B: Behavior> {
    started: Option<ResMut<'w, Events<StartedEvent<B>>>>,
    resumed: Option<ResMut<'w, Events<ResumedEvent<B>>>>,
    paused: Option<ResMut<'w, Events<PausedEvent<B>>>>,
    stopped: Option<ResMut<'w, Events<StoppedEvent<B>>>>,
}

impl<'w, B: Behavior> BehaviorEventWriter<'w, B> {
    pub(crate) fn send_started(&mut self, instance: Instance<B>) {
        if let Some(started) = &mut self.started {
            started.send(StartedEvent { instance });
        }
    }

    pub(crate) fn send_resumed(&mut self, instance: Instance<B>) {
        if let Some(resumed) = &mut self.resumed {
            resumed.send(ResumedEvent { instance });
        }
    }

    pub(crate) fn send_paused(&mut self, instance: Instance<B>) {
        if let Some(paused) = &mut self.paused {
            paused.send(PausedEvent { instance });
        }
    }

    pub(crate) fn send_stopped(&mut self, instance: Instance<B>, behavior: B) {
        if let Some(stopped) = &mut self.stopped {
            stopped.send(StoppedEvent { instance, behavior });
        }
    }
}

/// An event emitted when a [`Behavior`] is started.
#[derive(Event)]
pub struct StartedEvent<B: Behavior> {
    pub instance: Instance<B>,
}

impl<B: Behavior> StartedEvent<B> {
    /// Returns the [`Entity`] that started the [`Behavior`].
    pub fn entity(&self) -> Entity {
        self.instance.entity()
    }
}

/// An event emitted when a [`Behavior`] is resumed.
#[derive(Event)]
pub struct ResumedEvent<B: Behavior> {
    pub instance: Instance<B>,
}

impl<B: Behavior> ResumedEvent<B> {
    /// Returns the [`Entity`] that resumed the [`Behavior`].
    pub fn entity(&self) -> Entity {
        self.instance.entity()
    }
}

/// An event emitted when a [`Behavior`] is paused.
#[derive(Event)]
pub struct PausedEvent<B: Behavior> {
    pub instance: Instance<B>,
}

impl<B: Behavior> PausedEvent<B> {
    /// Returns the [`Entity`] that paused the [`Behavior`].
    pub fn entity(&self) -> Entity {
        self.instance.entity()
    }
}

/// An event emitted when a [`Behavior`] is stopped.
#[derive(Event)]
pub struct StoppedEvent<B: Behavior> {
    pub instance: Instance<B>,
    pub behavior: B,
}

impl<B: Behavior> StoppedEvent<B> {
    /// Returns the [`Entity`] that stopped the [`Behavior`].
    pub fn entity(&self) -> Entity {
        self.instance.entity()
    }
}

/// An [`EventReader`] for [`StartedEvent`]s.
pub type Started<'w, 's, B> = EventReader<'w, 's, StartedEvent<B>>;

/// An [`EventReader`] for [`ResumedEvent`]s.
pub type Resumed<'w, 's, B> = EventReader<'w, 's, ResumedEvent<B>>;

/// An [`EventReader`] for [`PausedEvent`]s.
pub type Paused<'w, 's, B> = EventReader<'w, 's, PausedEvent<B>>;

/// An [`EventReader`] for [`StoppedEvent`]s.
pub type Stopped<'w, 's, B> = EventReader<'w, 's, StoppedEvent<B>>;
