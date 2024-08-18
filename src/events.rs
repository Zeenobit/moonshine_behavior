use std::marker::PhantomData;

use bevy_ecs::{prelude::*, system::SystemParam};

use crate::Behavior;

#[doc(hidden)]
#[derive(SystemParam)]
pub struct BehaviorEventWriter<'w, B: Behavior> {
    started: EventWriter<'w, StartedEvent<B>>,
    resumed: EventWriter<'w, ResumedEvent<B>>,
    paused: EventWriter<'w, PausedEvent<B>>,
    stopped: EventWriter<'w, StoppedEvent<B>>,
}

impl<'w, B: Behavior> BehaviorEventWriter<'w, B> {
    pub(crate) fn send_started(&mut self, entity: Entity) {
        self.started.send(StartedEvent::new(entity));
    }

    pub(crate) fn send_resumed(&mut self, entity: Entity) {
        self.resumed.send(ResumedEvent::new(entity));
    }

    pub(crate) fn send_paused(&mut self, entity: Entity) {
        self.paused.send(PausedEvent::new(entity));
    }

    pub(crate) fn send_stopped(&mut self, entity: Entity, behavior: B) {
        self.stopped.send(StoppedEvent::new(entity, behavior));
    }
}

/// An event emitted when a [`Behavior`] is started.
#[derive(Event)]
pub struct StartedEvent<B: Behavior> {
    entity: Entity,
    marker: PhantomData<B>,
}

impl<B: Behavior> StartedEvent<B> {
    fn new(entity: Entity) -> Self {
        Self {
            entity,
            marker: PhantomData,
        }
    }

    /// Returns the [`Entity`] that started the [`Behavior`].
    pub fn entity(&self) -> Entity {
        self.entity
    }
}

/// An event emitted when a [`Behavior`] is resumed.
#[derive(Event)]
pub struct ResumedEvent<B: Behavior> {
    entity: Entity,
    marker: PhantomData<B>,
}

impl<B: Behavior> ResumedEvent<B> {
    fn new(entity: Entity) -> Self {
        Self {
            entity,
            marker: PhantomData,
        }
    }

    /// Returns the [`Entity`] that resumed the [`Behavior`].
    pub fn entity(&self) -> Entity {
        self.entity
    }
}

/// An event emitted when a [`Behavior`] is paused.
#[derive(Event)]
pub struct PausedEvent<B: Behavior> {
    entity: Entity,
    marker: PhantomData<B>,
}

impl<B: Behavior> PausedEvent<B> {
    fn new(entity: Entity) -> Self {
        Self {
            entity,
            marker: PhantomData,
        }
    }

    /// Returns the [`Entity`] that paused the [`Behavior`].
    pub fn entity(&self) -> Entity {
        self.entity
    }
}

/// An event emitted when a [`Behavior`] is stopped.
#[derive(Event)]
pub struct StoppedEvent<B: Behavior> {
    entity: Entity,
    behavior: B,
}

impl<B: Behavior> StoppedEvent<B> {
    fn new(entity: Entity, behavior: B) -> Self {
        Self { entity, behavior }
    }

    /// Returns the [`Entity`] that stopped the [`Behavior`].
    pub fn entity(&self) -> Entity {
        self.entity
    }

    /// Returns the [`Behavior`] that was stopped.
    pub fn behavior(&self) -> &B {
        &self.behavior
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
