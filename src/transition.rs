//! A [`Behavior`] is controlled using [`Transition`].
//!
//! Transitions are processed by the [`transition`] system.

use std::collections::VecDeque;
use std::fmt::Debug;

use bevy_ecs::component::Components;
use bevy_ecs::event::EntityComponentsTrigger;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_reflect::prelude::*;
use moonshine_kind::prelude::*;
use moonshine_util::prelude::*;

use crate::events::Start;
use crate::{
    Behavior, BehaviorHooks, BehaviorIndex, BehaviorItem, BehaviorMut, BehaviorMutItem, Memory,
};

pub use self::Transition::{Interrupt, Next, Previous};

/// A [`Component`] which controls transitions between [`Behavior`] states.
///
/// This component is automatically registered as a required component for all types
/// which implement the [`Behavior`] trait and and have their [`BehaviorPlugin`](crate::plugin::BehaviorPlugin) added.
#[derive(Component, Clone, Debug, Reflect)]
#[require(Expect<T>, Memory<T>)]
#[reflect(Component)]
pub enum Transition<T: Behavior> {
    #[doc(hidden)]
    None,
    /// Starts the next behavior.
    Next(T),
    /// Starts an [`Interruption`].
    Interrupt(Interruption<T>),
    /// Stops the current behavior and resumes the previous one.
    Previous,
}

impl<T: Behavior> Transition<T> {
    /// Returns `true` if there are no pending transitions.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    fn take(&mut self) -> Self {
        std::mem::replace(self, Transition::None)
    }
}

impl<T: Behavior> Default for Transition<T> {
    fn default() -> Self {
        Self::None
    }
}

/// A system which processes [`Behavior`] [`Transitions`](Transition).
#[allow(clippy::type_complexity)]
pub fn transition<T: Behavior>(
    components: &Components,
    mut query: Query<
        (Instance<T>, BehaviorMut<T>, Option<&mut TransitionQueue<T>>),
        Or<(Changed<Transition<T>>, With<TransitionQueue<T>>)>,
    >,
    mut commands: Commands,
) {
    for (instance, mut behavior, queue_opt) in &mut query {
        let entity = instance.entity();
        let component_id = components.valid_component_id::<T>().unwrap();
        let components = [component_id];

        if behavior.is_added() {
            behavior.invoke_start(None, commands.instance(instance));
            commands.queue(move |world: &mut World| {
                world.trigger_with(
                    Start {
                        entity,
                        index: BehaviorIndex::initial(),
                    },
                    EntityComponentsTrigger {
                        components: &components,
                    },
                );
            });
        }

        // Index of the stopped behavior, if applicable.
        let mut stop_index = None;

        let mut interrupt_queue = false;

        match behavior.transition.take() {
            Next(next) => {
                interrupt_queue = !behavior.push(instance, next, component_id, &mut commands);
            }
            Previous => {
                stop_index = Some(behavior.current_index());
                interrupt_queue = !behavior.pop(instance, component_id, &mut commands);
            }
            Interrupt(Interruption::Start(next)) => {
                behavior.interrupt(instance, next, component_id, &mut commands);
                interrupt_queue = true;
            }
            Interrupt(Interruption::Resume(index)) => {
                behavior.clear(instance, index, component_id, &mut commands);
                interrupt_queue = true;
            }
            _ => {}
        }

        let Some(queue) = queue_opt else {
            continue;
        };

        if interrupt_queue {
            debug!("{instance:?}: queue interrupted");
            commands.entity(entity).remove::<TransitionQueue<T>>();
        } else if queue.is_empty() {
            debug!("{instance:?}: queue finished");
            commands.entity(entity).remove::<TransitionQueue<T>>();
        } else {
            TransitionQueue::update(queue, instance, behavior, stop_index);
        }
    }
}

/// A specific kind of [`Transition`] which may stop active behaviors before activating a new one.
#[derive(Debug, Clone, Reflect)]
pub enum Interruption<T: Behavior> {
    /// An interruption which stops any behavior which [yields](Behavior::filter_yield)
    /// to the given behavior, and then starts it.
    Start(T),
    /// An interruption which resumes a behavior at the given index by stopping all other behaviors above it in the stack.
    Resume(BehaviorIndex),
}

#[doc(hidden)]
#[deprecated(since = "0.2.1", note = "use `Changed<Transition<T>>` instead")]
pub type TransitionChanged<T> = Or<(Changed<Transition<T>>, With<TransitionQueue<T>>)>;

/// Represents an error during [`transition`].
#[derive(Debug, PartialEq, Reflect)]
pub enum TransitionError<T: Behavior> {
    /// The given behavior was rejected by [`filter_next`](Behavior::filter_next).
    RejectedNext(T),
    /// Initial behavior may not be stopped.
    NoPrevious,
}

#[doc(hidden)]
#[deprecated(since = "0.3.1", note = "use `TransitionQueue` instead")]
pub type TransitionSequence<T> = TransitionQueue<T>;

/// A queue of transitions to be invoked automatically.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(Expect<Transition<T>>)]
pub struct TransitionQueue<T: Behavior> {
    queue: VecDeque<TransitionQueueItem<T>>,
    wait_for: Option<BehaviorIndex>,
}

impl<T: Behavior> TransitionQueue<T> {
    /// Creates a new transition sequence which starts all the given behaviors in given order.
    #[deprecated(since = "0.3.1", note = "use `TransitionQueue::chain` instead")]
    pub fn new(items: impl IntoIterator<Item = T>) -> Self {
        Self {
            queue: VecDeque::from_iter(items.into_iter().map(TransitionQueueItem::Start)),
            wait_for: None,
        }
    }

    /// Creates a new [`TransitionQueue`] which starts each given behavior in order.
    ///
    /// Unlike [`TransitionQueue::sequence`], a chain does not wait for the behaviors to stop
    /// and starts the behaviors on top of each other. This is useful when you wish to load a set of
    /// states onto the behavior stack (i.e. "Bird must Fly *and* Chirp").
    ///
    /// You must ensure that each transition in the chain is allowed (see [`Behavior::filter_next`]).
    /// The sequence will stop if any transition fails.
    pub fn chain(items: impl IntoIterator<Item = T>) -> Self {
        Self {
            queue: VecDeque::from_iter(items.into_iter().map(TransitionQueueItem::Start)),
            wait_for: None,
        }
    }

    /// Creates a new [`TransitionQueue`] which starts each given behavior in order,
    /// waiting for each one to stop before starting the next.
    ///
    /// Unlike [`TransitionQueue::chain`], a sequence waits for each behavior to stop before starting the next.
    /// This is useful when you want to queue some actions after each other (e.g. "Bird must Chirp, *then* Fly").
    ///
    /// You must ensure each transition from the current behavior to all elements in
    /// the sequence is allowed (see [`Behavior::filter_next`]).
    /// The sequence will stop if any transition fails.
    pub fn sequence(items: impl IntoIterator<Item = T>) -> Self {
        Self {
            queue: VecDeque::from_iter(items.into_iter().map(TransitionQueueItem::StartWait)),
            wait_for: None,
        }
    }

    /// Creates an empty [`TransitionQueue`].
    pub fn empty() -> Self {
        Self {
            queue: VecDeque::new(),
            wait_for: None,
        }
    }

    /// Creates a new [`TransitionQueue`] which starts with the given [`Behavior`].
    pub fn start(next: T) -> Self {
        let mut sequence = Self::empty();
        sequence.push(TransitionQueueItem::Start(next));
        sequence
    }

    /// Creates a new [`TransitionQueue`] which starts by stopping the current [`Behavior`].
    pub fn stop() -> Self {
        let mut sequence = Self::empty();
        sequence.push(TransitionQueueItem::Stop);
        sequence
    }

    /// Creates a new [`TransitionQueue`] which starts with the given [`Behavior`] and waits for it to stop.
    pub fn wait_for(next: T) -> Self {
        Self::empty().then_wait_for(next)
    }

    /// Returns `true` if the [`TransitionQueue`] is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Returns the number of transitions in the [`TransitionQueue`].
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Extends the [`TransitionQueue`] by starting the next [`Behavior`].
    pub fn then(mut self, next: T) -> Self {
        self.push(TransitionQueueItem::Start(next));
        self
    }

    /// Extends the [`TransitionQueue`] by starting the next [`Behavior`] if and only if `condition` is true.
    ///
    /// This is just a convenient shortcut useful when creating complex queues, for example:
    /// ```
    /// use bevy::prelude::*;
    /// use moonshine_behavior::prelude::*;
    ///
    /// #[derive(Component, Debug)]
    /// pub enum Foo {
    ///     Bar,
    ///     Baz,
    /// }
    ///
    /// impl Behavior for Foo {}
    ///
    /// fn make_queue(baz: bool) -> TransitionQueue<Foo> {
    ///     TransitionQueue::start(Foo::Bar).then_if(baz, Foo::Baz)
    /// }
    /// ```
    pub fn then_if(self, condition: bool, next: T) -> Self {
        if condition {
            return self.then(next);
        }
        self
    }

    /// Extends the [`TransitionQueue`] by starting the next [`Behavior`] and waiting for it to stop.
    pub fn then_wait_for(mut self, next: T) -> Self {
        self.push(TransitionQueueItem::StartWait(next));
        self
    }

    /// Extends the [`TransitionQueue`] by stopping the current [`Behavior`].
    pub fn then_stop(mut self) -> Self {
        self.push(TransitionQueueItem::Stop);
        self
    }

    fn push(&mut self, next: TransitionQueueItem<T>) {
        self.queue.push_back(next);
    }
}

impl<T: Behavior> TransitionQueue<T> {
    pub(crate) fn update(
        mut this: Mut<Self>,
        instance: Instance<T>,
        mut behavior: BehaviorMutItem<T>,
        stop_index: Option<BehaviorIndex>,
    ) {
        debug_assert!(!this.is_empty());

        if let Some(wait_index) = this.wait_for {
            if let Some(stop_index) = stop_index {
                if wait_index != stop_index {
                    return;
                }
            } else {
                return;
            }
        }

        debug!("{instance:?}: queue length = {:?}", this.len());

        if let Some(element) = this.queue.pop_front() {
            use TransitionQueueItem::*;
            match element {
                Start(next) => {
                    this.wait_for = None;
                    behavior.start(next);
                }
                StartWait(next) => {
                    this.wait_for = Some(behavior.current_index().next());
                    behavior.start(next);
                }
                Stop => {
                    this.wait_for = None;
                    behavior.stop();
                }
            }
        }
    }
}

#[derive(Debug, Reflect)]
enum TransitionQueueItem<T: Behavior> {
    Start(T),
    StartWait(T),
    Stop,
}
