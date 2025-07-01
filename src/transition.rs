use std::collections::VecDeque;
use std::fmt::Debug;

use bevy_ecs::component::Components;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_reflect::prelude::*;
use moonshine_kind::prelude::*;

use crate::events::OnStart;
use crate::{Behavior, BehaviorHooks, BehaviorIndex, BehaviorMut, BehaviorMutItem, Memory};

pub use self::Transition::{Interrupt, Next, Previous};

#[derive(Component, Clone, Debug, Reflect)]
#[require(Memory<T>)]
#[reflect(Component)]
pub enum Transition<T: Behavior> {
    None,
    Next(T),
    Interrupt(Interruption<T>),
    Previous,
}

impl<T: Behavior> Transition<T> {
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

pub fn transition<T: Behavior>(
    mut components: &Components,
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
    for (instance, mut behavior, sequence_opt) in &mut query {
        if behavior.current.is_added() {
            // Memory must be empty when the component is added
            debug_assert!(behavior.memory.is_empty());

            // Send start event for the initial behavior
            behavior.invoke_start(None, commands.instance(instance));
            let id = components.valid_component_id::<T>().unwrap();
            commands.trigger_targets(
                OnStart {
                    index: BehaviorIndex::initial(),
                },
                (*instance, id),
            );
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

#[deprecated(since = "0.2.1", note = "use `Changed<Transition<T>>` instead")]
pub type TransitionChanged<T> = Or<(Changed<Transition<T>>, With<TransitionSequence<T>>)>;

#[derive(Debug, PartialEq, Reflect)]
pub enum TransitionError<T: Behavior> {
    RejectedNext(T),
    NoPrevious,
}

#[derive(Debug, Clone, Reflect)]
pub enum Interruption<T: Behavior> {
    Start(T),
    Resume(BehaviorIndex),
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TransitionSequence<T: Behavior> {
    queue: VecDeque<TransitionSequenceElement<T>>,
    wait_index: Option<BehaviorIndex>,
}

impl<T: Behavior> TransitionSequence<T> {
    pub fn new(items: impl IntoIterator<Item = T>) -> Self {
        Self {
            queue: VecDeque::from_iter(items.into_iter().map(TransitionSequenceElement::Start)),
            wait_index: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            queue: VecDeque::new(),
            wait_index: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn start(next: T) -> Self {
        let mut sequence = Self::empty();
        sequence.push(TransitionSequenceElement::Start(next));
        sequence
    }

    pub fn wait_for(next: T) -> Self {
        Self::empty().then_wait_for(next)
    }

    pub fn then(mut self, next: T) -> Self {
        self.push(TransitionSequenceElement::Start(next));
        self
    }

    pub fn then_wait_for(mut self, next: T) -> Self {
        self.push(TransitionSequenceElement::StartWait(next));
        self
    }

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
