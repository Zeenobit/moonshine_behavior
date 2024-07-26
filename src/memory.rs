use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

use crate::Behavior;

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

    pub(crate) fn push(&mut self, behavior: B) {
        self.0.push(behavior)
    }

    pub(crate) fn pop(&mut self) -> Option<B> {
        self.0.pop()
    }
}

impl<B: Behavior> Default for Memory<B> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
