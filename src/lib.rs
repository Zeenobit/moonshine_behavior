use std::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    marker::PhantomData,
    mem::{replace, swap},
    ops::{Deref, DerefMut},
};

use bevy_app::{App, Plugin};
use bevy_ecs::{prelude::*, query::QueryData, system::SystemParam};
use bevy_reflect::{FromReflect, GetTypeRegistration, Reflect, TypePath};
use bevy_utils::tracing::{debug, error, warn};
use moonshine_util::future::{Future, Promise};

pub mod prelude {
    pub use crate::{
        BehaviorPlugin, {transition, InvalidTransition, TransitionResult},
        {Behavior, BehaviorBundle}, {BehaviorMut, BehaviorRef},
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

impl<B: Behavior + FromReflect + TypePath + GetTypeRegistration> Plugin for BehaviorPlugin<B> {
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
/// #[derive(Component, Default, Debug, Reflect)]
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
#[derive(Bundle, Default)]
pub struct BehaviorBundle<B: Behavior> {
    behavior: B,
    memory: Memory<B>,
    transition: Transition<B>,
}

impl<B: Behavior + Clone> Clone for BehaviorBundle<B> {
    fn clone(&self) -> Self {
        Self {
            behavior: self.behavior.clone(),
            memory: self.memory.clone(),
            transition: self.transition.clone(),
        }
    }
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
            transition: Transition::Started, // Initial Behavior
        }
    }

    /// Tries to start the given [`Behavior`] as the next one immediately after insertion.
    pub fn try_start(&mut self, next: B) -> Future<TransitionResult<B>> {
        let (transition, future) = Transition::next(next);
        self.transition = transition;
        future
    }
}

impl<B: Behavior> From<B> for BehaviorBundle<B> {
    fn from(behavior: B) -> Self {
        Self::new(behavior)
    }
}

/// A [`QueryData`] used to query a [`Behavior`].
#[derive(QueryData)]
pub struct BehaviorRef<B: Behavior> {
    behavior: Ref<'static, B>,
    memory: &'static Memory<B>,
    transition: &'static Transition<B>,
}

impl<B: Behavior> BehaviorRefItem<'_, B> {
    /// Returns a reference to the current [`Behavior`].
    pub fn get(&self) -> &B {
        &self.behavior
    }

    /// Returns `true` if the current [`Behavior`] was changed since last query.
    pub fn is_changed(&self) -> bool {
        self.behavior.is_changed()
    }

    /// Returns `true` if the current [`Behavior`] was just started.
    pub fn is_started(&self) -> bool {
        matches!(self.transition, Transition::Started)
    }

    /// Returns `true` if the current [`Behavior`] was just resumed.
    pub fn is_resumed(&self) -> bool {
        matches!(self.transition, Transition::Resumed)
    }

    /// Returns `true` if the current [`Behavior`] is active and not in a transition.
    pub fn is_stable(&self) -> bool {
        matches!(self.transition, Transition::Stable)
    }

    /// Returns a reference to the current [`Behavior`] if it was changed since last query.
    pub fn get_changed(&self) -> Option<&B> {
        self.is_changed().then(|| self.get())
    }

    /// Returns `true` if a [`Transition`] is currently in progress.
    pub fn has_transition(&self) -> bool {
        use Transition::*;
        !matches!(self.transition, Stable | Started | Resumed)
    }

    /// Returns a reference to the previous [`Behavior`], if it exists.
    pub fn previous(&self) -> Option<&B> {
        self.memory.previous()
    }

    /// Returns a reference to the [`Behavior`] [`Memory`].
    pub fn memory(&self) -> &Memory<B> {
        self.memory
    }
}

impl<B: Behavior> Deref for BehaviorRefItem<'_, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.behavior
    }
}

impl<B: Behavior> Borrow<B> for BehaviorRefItem<'_, B> {
    fn borrow(&self) -> &B {
        &self.behavior
    }
}

/// A mutable [`QueryData`] used to query and manipulate a [`Behavior`].
#[derive(QueryData)]
#[query_data(mutable)]
pub struct BehaviorMut<B: Behavior> {
    behavior: Mut<'static, B>,
    memory: &'static Memory<B>,
    transition: &'static mut Transition<B>,
}

impl<B: Behavior> BehaviorMutReadOnlyItem<'_, B> {
    /// Returns a reference to the current [`Behavior`].
    pub fn get(&self) -> &B {
        &self.behavior
    }

    /// Returns `true` if the current [`Behavior`] was changed since last query.
    pub fn is_changed(&self) -> bool {
        self.behavior.is_changed()
    }

    /// Returns `true` if the current [`Behavior`] was just started.
    pub fn is_started(&self) -> bool {
        matches!(self.transition, Transition::Started)
    }

    /// Returns `true` if the current [`Behavior`] was just resumed.
    pub fn is_resumed(&self) -> bool {
        matches!(self.transition, Transition::Resumed)
    }

    /// Returns `true` if the current [`Behavior`] is active and not in a transition.
    pub fn is_stable(&self) -> bool {
        matches!(self.transition, Transition::Stable)
    }

    /// Returns a reference to the current [`Behavior`] if it was changed since last query.
    pub fn get_changed(&self) -> Option<&B> {
        self.is_changed().then(|| self.get())
    }

    /// Returns `true` if a [`Transition`] is currently in progress.
    pub fn has_transition(&self) -> bool {
        use Transition::*;
        !matches!(self.transition, Stable | Started | Resumed)
    }

    /// Returns a reference to the previous [`Behavior`], if it exists.
    pub fn previous(&self) -> Option<&B> {
        self.memory.previous()
    }

    /// Returns a reference to the [`Behavior`] [`Memory`].
    pub fn memory(&self) -> &Memory<B> {
        self.memory
    }
}

impl<B: Behavior> Deref for BehaviorMutReadOnlyItem<'_, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.behavior
    }
}

impl<B: Behavior> Borrow<B> for BehaviorMutReadOnlyItem<'_, B> {
    fn borrow(&self) -> &B {
        &self.behavior
    }
}

impl<B: Behavior> BehaviorMutItem<'_, B> {
    /// Returns a reference to the current [`Behavior`].
    pub fn get(&self) -> &B {
        &self.behavior
    }

    /// Returns a mutable reference to the current [`Behavior`].
    pub fn get_mut(&mut self) -> &mut B {
        &mut self.behavior
    }

    /// Returns `true` if the current [`Behavior`] was changed since last query.
    pub fn is_changed(&self) -> bool {
        self.behavior.is_changed()
    }

    /// Returns `true` if the current [`Behavior`] was just started.
    pub fn is_started(&self) -> bool {
        matches!(*self.transition, Transition::Started)
    }

    /// Returns `true` if the current [`Behavior`] was just resumed.
    pub fn is_resumed(&self) -> bool {
        matches!(*self.transition, Transition::Resumed)
    }

    /// Returns `true` if the current [`Behavior`] is active and not in a transition.
    pub fn is_stable(&self) -> bool {
        matches!(*self.transition, Transition::Stable)
    }

    /// Returns a reference to the current [`Behavior`] if it was changed since last query.
    pub fn get_changed(&self) -> Option<&B> {
        self.is_changed().then(|| self.get())
    }

    /// Returns a mutable reference to the current [`Behavior`] if it was changed since last query.
    pub fn get_changed_mut(&mut self) -> Option<&mut B> {
        self.is_changed().then(|| self.get_mut())
    }

    /// Returns `true` if a [`Transition`] is currently in progress.
    pub fn has_transition(&self) -> bool {
        use Transition::*;
        !matches!(*self.transition, Stable | Started | Resumed)
    }

    /// Tries to start the given [`Behavior`] as the next one.
    ///
    /// This only sets the behavior [`Transition`], and does not immediately modify the behavior.
    /// The next behavior will only start if it is allowed to by the [`transition()`] system.
    /// Otherwise, the transition is ignored.
    pub fn try_start(&mut self, next: B) -> Future<TransitionResult<B>> {
        let (transition, future) = Transition::next(next);
        let previous = replace(self.transition.as_mut(), transition);
        if !matches!(previous, Transition::Stable) {
            warn!(
                "transition override: {previous:?} -> {:?}",
                self.transition.as_ref(),
            );
        }
        future
    }

    /// Stops the current [`Behavior`] and tries to resume the previous one.
    ///
    /// If the previous behavior is not resumable, the behavior before it is tried,
    /// and so on until a resumable behavior is found.
    ///
    /// If the current behavior is the initial one, it does nothing.
    pub fn stop(&mut self) {
        let previous = replace(self.transition.as_mut(), Transition::previous());
        if !matches!(previous, Transition::Stable) {
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
        let previous = replace(self.transition.as_mut(), Transition::reset());
        if !matches!(previous, Transition::Stable) {
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

    /// Returns a reference to the [`Behavior`] [`Memory`].
    pub fn memory(&self) -> &Memory<B> {
        self.memory
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

impl<B: Behavior> Borrow<B> for BehaviorMutItem<'_, B> {
    fn borrow(&self) -> &B {
        self.behavior.as_ref()
    }
}

impl<B: Behavior> BorrowMut<B> for BehaviorMutItem<'_, B> {
    fn borrow_mut(&mut self) -> &mut B {
        self.behavior.as_mut()
    }
}

/// A [`Component`] which stores a stack of paused [`Behavior`] states to be resumed later.
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct Memory<B: Behavior>(Vec<B>);

impl<B: Behavior> Memory<B> {
    /// Returns the number of paused [`Behavior`] states in the stack.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the paused [`Behavior`] states in the stack.
    ///
    /// The iterator starts from the most recently paused state (previous).
    pub fn iter(&self) -> impl Iterator<Item = &B> {
        self.0.iter().rev()
    }

    /// Returns `true` if the stack contains the given [`Behavior`] state.
    pub fn contains(&self, behavior: &B) -> bool
    where
        B: PartialEq,
    {
        self.0.contains(behavior)
    }

    /// Returns a reference to the previous [`Behavior`] state, if it exists.
    pub fn previous(&self) -> Option<&B> {
        self.0.last()
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
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub enum Transition<B: Behavior> {
    #[default]
    Stable,
    Started,
    Resumed,
    #[reflect(ignore)]
    Next(B, #[reflect(ignore)] Promise<TransitionResult<B>>),
    #[reflect(ignore)]
    Previous,
    #[reflect(ignore)]
    Reset,
}

impl<B: Behavior> Transition<B> {
    pub fn next(next: B) -> (Self, Future<TransitionResult<B>>) {
        let (promise, future) = Promise::start();
        let transition = Self::Next(next, promise);
        (transition, future)
    }

    pub fn previous() -> Self {
        Self::Previous
    }

    pub fn reset() -> Self {
        Self::Reset
    }

    fn take(&mut self) -> Self {
        let mut t = Self::Stable;
        swap(self, &mut t);
        t
    }

    fn clone(&self) -> Self
    where
        B: Clone,
    {
        use Transition::*;
        match self {
            Stable => Stable,
            Started => Started,
            Resumed => Resumed,
            Next(next, _) => Next(next.clone(), Promise::new()),
            Previous => Previous,
            Reset => Reset,
        }
    }
}

impl<B: Behavior> Debug for Transition<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Transition::*;
        match self {
            Stable => write!(f, "Stable"),
            Started => write!(f, "Started"),
            Resumed => write!(f, "Resumed"),
            Next(arg0, _) => f.debug_tuple("Next").field(arg0).finish(),
            Previous => write!(f, "Previous"),
            Reset => write!(f, "Reset"),
        }
    }
}

pub type TransitionResult<B> = Result<(), InvalidTransition<B>>;

#[derive(Debug)]
pub struct InvalidTransition<B: Behavior>(pub B);

#[doc(hidden)]
#[derive(SystemParam)]
pub struct Events<'w, B: Behavior> {
    started: EventWriter<'w, StartedEvent<B>>,
    resumed: EventWriter<'w, ResumedEvent<B>>,
    paused: EventWriter<'w, PausedEvent<B>>,
    stopped: EventWriter<'w, StoppedEvent<B>>,
}

impl<'w, B: Behavior> Events<'w, B> {
    fn send_started(&mut self, entity: Entity) {
        self.started.send(StartedEvent::new(entity));
    }

    fn send_resumed(&mut self, entity: Entity) {
        self.resumed.send(ResumedEvent::new(entity));
    }

    fn send_paused(&mut self, entity: Entity) {
        self.paused.send(PausedEvent::new(entity));
    }

    fn send_stopped(&mut self, entity: Entity, behavior: B) {
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
    mut query: Query<(Entity, &mut B, &mut Memory<B>, &mut Transition<B>)>,
    mut events: Events<B>,
) {
    for (entity, current, memory, mut transition) in &mut query {
        use Transition::*;

        // Do not mutate the transition if stable:
        if matches!(*transition, Stable) {
            continue;
        }

        match transition.take() {
            Next(next, promise) => {
                let value = push(entity, next, current, memory, &mut events);
                if value.is_ok() {
                    *transition = Started;
                }
                promise.set(value);
            }
            Previous => {
                if pop(entity, current, memory, &mut events) {
                    *transition = Resumed;
                }
            }
            Reset => {
                if reset(entity, current, memory, &mut events) {
                    *transition = Resumed;
                }
            }
            Started | Resumed => {
                *transition = Stable;
            }
            Stable => unreachable!(),
        }
    }
}

fn push<B: Behavior>(
    entity: Entity,
    mut next: B,
    mut current: Mut<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut Events<B>,
) -> TransitionResult<B> {
    if current.allows_next(&next) {
        debug!("{entity:?}: {:?} -> {next:?}", *current);
        let behavior = {
            swap(current.as_mut(), &mut next);
            next
        };
        if behavior.is_resumable() {
            events.send_paused(entity);
            memory.push(behavior);
        } else {
            events.send_stopped(entity, behavior);
        }
        events.send_started(entity);
        Ok(())
    } else {
        warn!("{entity:?}: {:?} -> {next:?} is not allowed", *current);
        Err(InvalidTransition(next))
    }
}

fn pop<B: Behavior>(
    entity: Entity,
    mut current: Mut<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut Events<B>,
) -> bool {
    if let Some(mut next) = memory.pop() {
        debug!("{entity:?}: {:?} -> {next:?}", *current);
        let behavior = {
            swap(current.as_mut(), &mut next);
            next
        };
        events.send_resumed(entity);
        events.send_stopped(entity, behavior);
        true
    } else {
        error!("{entity:?}: {:?} -> None is not allowed", *current);
        false
    }
}

fn reset<B: Behavior>(
    entity: Entity,
    mut current: Mut<B>,
    mut memory: Mut<Memory<B>>,
    events: &mut Events<B>,
) -> bool {
    while memory.len() > 1 {
        let behavior = memory.pop().unwrap();
        events.send_stopped(entity, behavior);
    }

    if let Some(mut next) = memory.pop() {
        debug!("{entity:?}: {:?} -> {next:?}", *current);
        let behavior = {
            swap(current.as_mut(), &mut next);
            next
        };
        events.send_resumed(entity);
        events.send_stopped(entity, behavior);
        true
    } else {
        warn!("{entity:?}: {:?} -> {:?} is redundant", *current, *current);
        false
    }
}
