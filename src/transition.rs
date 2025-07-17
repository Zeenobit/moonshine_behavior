//! A [`Behavior`] is controlled using [`Transition`].
//!
//! Transitions are processed by the [`transition`] system.

use std::collections::VecDeque;
use std::fmt::Debug;

use bevy_ecs::component::Components;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_reflect::prelude::*;
use moonshine_kind::prelude::*;

use crate::events::{OnActivate, OnStart};
use crate::{Behavior, BehaviorHooks, BehaviorIndex, BehaviorMut, BehaviorMutItem, Memory};

pub use self::Transition::{Interrupt, Next, Previous};

/// A [`Component`] which controls transitions between [`Behavior`] states.
///
/// This component is automatically registered as a required component for all types
/// which implement the [`Behavior`] trait and and have their [`BehaviorPlugin`](crate::plugin::BehaviorPlugin) added.
#[derive(Component, Clone, Debug, Reflect)]
#[require(Memory<T>)]
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
pub fn transition<T: Behavior>(
    components: &Components,
    mut query: Query<
        (
            Instance<T>,
            BehaviorMut<T>,
            Option<&mut TransitionSequence<T>>,
        ),
        Or<(Changed<Transition<T>>, With<TransitionSequence<T>>)>,
    >,
    mut commands: Commands,
) {
    let id = components.valid_component_id::<T>().unwrap();

    for (instance, mut behavior, sequence_opt) in &mut query {
        if behavior.current.is_added() {
            let index = BehaviorIndex::initial();
            behavior[index].invoke_start(None, commands.instance(instance));
            commands.trigger_targets(OnStart { index }, (*instance, id));
            commands.trigger_targets(
                OnActivate {
                    index,
                    resume: false,
                },
                (*instance, id),
            );

            for (index, current) in behavior.enumerate().skip(1) {
                let previous = &behavior[index.previous()];
                previous.invoke_pause(current, commands.instance(instance));
                behavior[index].invoke_start(Some(previous), commands.instance(instance));
                commands.trigger_targets(OnStart { index }, (*instance, id));
                commands.trigger_targets(
                    OnActivate {
                        index,
                        resume: false,
                    },
                    (*instance, id),
                );
            }
        }

        // Index of the stopped behavior, if applicable.
        let mut stop_index = None;

        let mut interrupt_sequence = false;

        match behavior.transition.take() {
            Next(next) => {
                interrupt_sequence = !behavior.push(instance, next, components, &mut commands);
            }
            Previous => {
                stop_index = Some(behavior.index());
                interrupt_sequence = !behavior.pop(instance, components, &mut commands);
            }
            Interrupt(Interruption::Start(next)) => {
                behavior.interrupt(instance, next, components, &mut commands);
                interrupt_sequence = true;
            }
            Interrupt(Interruption::Resume(index)) => {
                behavior.clear(instance, index, components, &mut commands);
                interrupt_sequence = true;
            }
            _ => {}
        }

        let Some(sequence) = sequence_opt else {
            continue;
        };

        if interrupt_sequence {
            debug!("{instance:?}: sequence interrupted");
            commands.entity(*instance).remove::<TransitionSequence<T>>();
        } else if sequence.is_empty() {
            debug!("{instance:?}: sequence finished");
            commands.entity(*instance).remove::<TransitionSequence<T>>();
        } else {
            TransitionSequence::update(sequence, instance, behavior, stop_index);
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
pub type TransitionChanged<T> = Or<(Changed<Transition<T>>, With<TransitionSequence<T>>)>;

/// Represents an error during [`transition`].
#[derive(Debug, PartialEq, Reflect)]
pub enum TransitionError<T: Behavior> {
    /// The given behavior was rejected by [`filter_next`](Behavior::filter_next).
    RejectedNext(T),
    /// Initial behavior may not be stopped.
    NoPrevious,
}

/// A queue of transitions to start automated behavior sequences.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TransitionSequence<T: Behavior> {
    queue: VecDeque<TransitionSequenceElement<T>>,
    wait_index: Option<BehaviorIndex>,
}

impl<T: Behavior> TransitionSequence<T> {
    /// Creates a new transition sequence which starts all the given behaviors in given order.
    pub fn new(items: impl IntoIterator<Item = T>) -> Self {
        Self {
            queue: VecDeque::from_iter(items.into_iter().map(TransitionSequenceElement::Start)),
            wait_index: None,
        }
    }

    /// Creates an empty transition sequence.
    pub fn empty() -> Self {
        Self {
            queue: VecDeque::new(),
            wait_index: None,
        }
    }

    /// Creates a new transition sequence which starts with the given behavior.
    pub fn start(next: T) -> Self {
        let mut sequence = Self::empty();
        sequence.push(TransitionSequenceElement::Start(next));
        sequence
    }

    /// Creates a new transition sequence which starts with the given behavior and waits for it to finish.
    pub fn wait_for(next: T) -> Self {
        Self::empty().then_wait_for(next)
    }

    /// Returns `true` if the sequence is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Returns the number of transitions in the sequence.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Starts the next behavior in the sequence.
    pub fn then(mut self, next: T) -> Self {
        self.push(TransitionSequenceElement::Start(next));
        self
    }

    /// Starts the next behavior in the sequence and waits for it to finish.
    pub fn then_wait_for(mut self, next: T) -> Self {
        self.push(TransitionSequenceElement::StartWait(next));
        self
    }

    /// Stops the current behavior.
    pub fn then_stop(mut self) -> Self {
        self.push(TransitionSequenceElement::Stop);
        self
    }

    fn push(&mut self, next: TransitionSequenceElement<T>) {
        self.queue.push_back(next);
    }
}

impl<T: Behavior> TransitionSequence<T> {
    pub(crate) fn update(
        mut this: Mut<Self>,
        instance: Instance<T>,
        mut behavior: BehaviorMutItem<T>,
        stop_index: Option<BehaviorIndex>,
    ) {
        debug_assert!(!this.queue.is_empty());

        if let Some(wait_index) = this.wait_index {
            if let Some(stop_index) = stop_index {
                if wait_index != stop_index {
                    return;
                }
            } else {
                return;
            }
        }

        debug!("{instance:?}: sequence steps = {:?}", this.queue.len());

        if let Some(element) = this.queue.pop_front() {
            use TransitionSequenceElement::*;
            match element {
                Start(next) => {
                    this.wait_index = None;
                    behavior.start(next);
                }
                StartWait(next) => {
                    this.wait_index = Some(behavior.index().next());
                    behavior.start(next);
                }
                Stop => {
                    this.wait_index = None;
                    behavior.stop();
                }
            }
        }
    }
}

#[derive(Debug, Reflect)]
enum TransitionSequenceElement<T: Behavior> {
    Start(T),
    StartWait(T),
    Stop,
}
