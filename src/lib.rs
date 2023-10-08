use std::{
    fmt::Debug,
    marker::PhantomData,
    mem::{replace, swap},
    ops::{Deref, DerefMut},
};

use bevy_app::{App, Plugin};
use bevy_ecs::{prelude::*, query::WorldQuery, system::SystemParam};
use bevy_reflect::{FromReflect, Reflect, TypePath};
use bevy_utils::tracing::{debug, error, warn};

pub mod prelude {
    pub use crate::{
        transition, BehaviorPlugin, {Behavior, BehaviorBundle}, {BehaviorMut, BehaviorRef},
        {Paused, Resumed, Started, Stopped},
        {PausedEvent, ResumedEvent, StartedEvent, StoppedEvent},
    };
}

#[cfg(test)]
mod tests;

/// A [`Plugin`] which registers a [`Behavior`] type with its [`App`].
pub struct BehaviorPlugin<B: Behavior>(PhantomData<B>);

impl<B: Behavior> Default for BehaviorPlugin<B> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<B: Behavior + FromReflect + TypePath> Plugin for BehaviorPlugin<B> {
    fn build(&self, app: &mut App) {
        app.add_event::<StartedEvent<B>>()
            .add_event::<PausedEvent<B>>()
            .add_event::<ResumedEvent<B>>()
            .add_event::<StoppedEvent<B>>()
            .register_type::<Memory<B>>()
            .register_type::<Vec<B>>()
            .register_type::<Transition<B>>();
    }
}

/// A [`Component`] which represents some state of its [`Entity`].
///
/// # Usage
/// A [`Behavior`] is typically implemented as an `enum` type.
///
/// # Example
/// ```
/// use bevy::prelude::*;
/// use moonshine_behavior::prelude::*;
///
/// #[derive(Component, Default, Debug, Reflect, FromReflect)]
/// #[reflect(Component)]
/// enum Bird {
///     #[default]
///     Idle,
///     Fly,
///     Sleep,
///     Chirp,
/// }
///
/// use Bird::*;
///
/// impl Behavior for Bird {
///     fn allows_next(&self, next: &Self) -> bool {
///         match self {
///             Idle => matches!(next, Sleep | Fly | Chirp),
///             Fly => matches!(next, Chirp),
///             Sleep | Chirp => false,
///         }
///     }
///
///     fn is_resumable(&self) -> bool {
///         matches!(self, Idle | Fly)
///     }
/// }
///
/// fn spawn_bird(mut commands: Commands) {
///     commands.spawn(BehaviorBundle::<Bird>::default());
/// }
///
/// fn chirp(mut bird: Query<BehaviorMut<Bird>>) {
///     bird.single_mut().try_start(Chirp);
/// }
///
/// fn is_chirping_while_flying(bird: Query<BehaviorRef<Bird>>) -> bool {
///     let behavior = bird.single();
///     matches!(*behavior, Chirp) && matches!(behavior.previous(), Some(Fly))
/// }
/// ```
pub trait Behavior: Component + Debug {
    /// Returns `true` if some next [`Behavior`] is allowed to be started after this one.
    ///
    /// By default, any behavior is allowed to start after any other behavior.
    fn allows_next(&self, _next: &Self) -> bool {
        true
    }

    /// Returns `true` if this [`Behavior`] may be resumed after it has been paused.
    ///
    /// By default, all behaviors are resumable.
    fn is_resumable(&self) -> bool {
        true
    }
}

/// A [`Bundle`] which contains a [`Behavior`] and other required components for it to function.
#[derive(Bundle, Default, Clone)]
pub struct BehaviorBundle<B: Behavior> {
    behavior: B,
    memory: Memory<B>,
    transition: Transition<B>,
}

impl<B: Behavior> BehaviorBundle<B> {
    pub fn new(behavior: B) -> Self {
        assert!(
            behavior.is_resumable(),
            "initial behavior must be resumable"
        );
        Self {
            behavior,
            memory: Memory::default(),
            transition: Transition::default(),
        }
    }

    /// Tries to start the given [`Behavior`] as the next one immediately after insertion.
    pub fn try_start(mut self, next: B) -> Self {
        self.transition = Next(next);
        self
    }
}

/// A [`WorldQuery`] used to query a [`Behavior`].
#[derive(WorldQuery)]
pub struct BehaviorRef<B: Behavior> {
    behavior: &'static B,
    memory: &'static Memory<B>,
}

impl<B: Behavior> BehaviorRefItem<'_, B> {
    pub fn get(&self) -> &B {
        self.behavior
    }

    pub fn previous(&self) -> Option<&B> {
        self.memory.previous()
    }
}

impl<B: Behavior> Deref for BehaviorRefItem<'_, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        self.behavior
    }
}

/// A mutable [`WorldQuery`] used to query and manipulate a [`Behavior`].
#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct BehaviorMut<B: Behavior> {
    behavior: &'static mut B,
    memory: &'static Memory<B>,
    transition: &'static mut Transition<B>,
}

impl<B: Behavior> BehaviorMutReadOnlyItem<'_, B> {
    /// Returns a reference to the current [`Behavior`].
    pub fn get(&self) -> &B {
        self.behavior
    }

    /// Returns a reference to the previous [`Behavior`], if it exists.
    pub fn previous(&self) -> Option<&B> {
        self.memory.previous()
    }
}

impl<B: Behavior> Deref for BehaviorMutReadOnlyItem<'_, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        self.behavior
    }
}

impl<B: Behavior> BehaviorMutItem<'_, B> {
    pub fn get(&self) -> &B {
        &self.behavior
    }

    pub fn get_mut(&mut self) -> &mut B {
        &mut self.behavior
    }

    /// Tries to start the given [`Behavior`] as the next one.
    ///
    /// This only sets the behavior [`Transition`], and does not immediately modify the behavior.
    /// The next behavior will only start if it is allowed to by the [`transition()`] system.
    /// Otherwise, the transition is ignored.
    pub fn try_start(&mut self, next: B) {
        let previous = replace(self.transition.as_mut(), Next(next));
        if !matches!(previous, Transition::Empty) {
            warn!(
                "transition override: {previous:?} -> {:?}",
                self.transition.as_ref(),
            );
        }
    }

    /// Stops the current [`Behavior`] and tries to resume the previous one.
    ///
    /// If the previous behavior is not resumable, the behavior before it is tried,
    /// and so on until a resumable behavior is found.
    ///
    /// If the current behavior is the initial one, it does nothing.
    pub fn stop(&mut self) {
        let previous = replace(self.transition.as_mut(), Previous);
        if !matches!(previous, Transition::Empty) {
            warn!(
                "transition override: {previous:?} -> {:?}",
                self.transition.as_ref(),
            );
        }
    }

    /// Stops the current [`Behavior`] and resumes the initial one.
    ///
    /// If the current behavior is the initial one, it does nothing.
    pub fn reset(&mut self) {
        let previous = replace(self.transition.as_mut(), Reset);
        if !matches!(previous, Transition::Empty) {
            warn!(
                "transition override: {previous:?} -> {:?}",
                self.transition.as_ref(),
            );
        }
    }

    /// Returns a reference to the previous [`Behavior`], if it exists.
    pub fn previous(&self) -> Option<&B> {
        self.memory.previous()
    }
}

impl<B: Behavior> Deref for BehaviorMutItem<'_, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        self.behavior.as_ref()
    }
}

impl<B: Behavior> DerefMut for BehaviorMutItem<'_, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.behavior.as_mut()
    }
}

/// A [`Component`] which stores paused [`Behavior`] components to be resumed later.
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct Memory<B: Behavior>(Vec<B>);

impl<B: Behavior> Memory<B> {
    fn previous(&self) -> Option<&B> {
        self.0.last()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn push(&mut self, behavior: B) {
        self.0.push(behavior)
    }

    fn pop(&mut self) -> Option<B> {
        self.0.pop()
    }
}

impl<B: Behavior> Default for Memory<B> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

/// A [`Component`] used to trigger [`Behavior`] transitions.
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component)]
pub enum Transition<B: Behavior> {
    #[default]
    Empty,
    #[reflect(ignore)]
    Next(B),
    #[reflect(ignore)]
    Previous,
    #[reflect(ignore)]
    Reset,
}

pub use Transition::{Next, Previous, Reset};

impl<B: Behavior> Transition<B> {
    fn take(&mut self) -> Self {
        let mut t = Self::Empty;
        swap(self, &mut t);
        t
    }
}

#[doc(hidden)]
#[derive(SystemParam)]
pub struct Events<'w, B: Behavior> {
    started: EventWriter<'w, StartedEvent<B>>,
    resumed: EventWriter<'w, ResumedEvent<B>>,
    paused: EventWriter<'w, PausedEvent<B>>,
    stopped: EventWriter<'w, StoppedEvent<B>>,
}

impl<'w, B: Behavior> Events<'w, B> {
    pub fn started(&mut self, entity: Entity) {
        self.started.send(StartedEvent::new(entity));
    }

    pub fn resumed(&mut self, entity: Entity) {
        self.resumed.send(ResumedEvent::new(entity));
    }

    pub fn paused(&mut self, entity: Entity) {
        self.paused.send(PausedEvent::new(entity));
    }

    pub fn stopped(&mut self, entity: Entity, behavior: B) {
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

/// A [`System`] which triggers [`Behavior`] transitions.
#[allow(clippy::type_complexity)]
pub fn transition<B: Behavior>(
    mut query: Query<(Entity, &mut B, &mut Memory<B>, &mut Transition<B>), Changed<Transition<B>>>,
    mut events: Events<B>,
) {
    for (entity, current, memory, mut transition) in &mut query {
        match transition.bypass_change_detection().take() {
            Next(next) => push(entity, next, current, memory, &mut events),
            Previous => pop(entity, current, memory, &mut events),
            Reset => reset(entity, current, memory, &mut events),
            _ => (),
        }
    }
}

fn push<B: Behavior>(
    entity: Entity,
    mut next: B,
    mut current: Mut<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut Events<B>,
) {
    if current.allows_next(&next) {
        debug!("{entity:?}: {:?} -> {next:?}", *current);
        let behavior = {
            swap(current.as_mut(), &mut next);
            next
        };
        if behavior.is_resumable() {
            events.paused(entity);
            memory.push(behavior);
        } else {
            events.stopped(entity, behavior);
        }
        events.started(entity);
    } else {
        warn!("{entity:?}: {:?} -> {next:?} is not allowed", *current);
    }
}

fn pop<B: Behavior>(
    entity: Entity,
    mut current: Mut<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut Events<B>,
) {
    if let Some(mut next) = memory.pop() {
        debug!("{entity:?}: {:?} -> {next:?}", *current);
        let behavior = {
            swap(current.as_mut(), &mut next);
            next
        };
        events.resumed(entity);
        events.stopped(entity, behavior);
    } else {
        error!("{entity:?}: {:?} -> None is not allowed", *current);
    }
}

fn reset<B: Behavior>(
    entity: Entity,
    mut current: Mut<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut Events<B>,
) {
    while memory.len() > 1 {
        let behavior = memory.pop().unwrap();
        events.stopped(entity, behavior);
    }

    if let Some(mut next) = memory.pop() {
        debug!("{entity:?}: {:?} -> {next:?}", *current);
        swap(current.as_mut(), &mut next);
        events.resumed(entity);
    } else {
        warn!("{entity:?}: {:?} -> {:?} is redundant", *current, *current);
    }
}
