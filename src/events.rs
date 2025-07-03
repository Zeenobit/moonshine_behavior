use bevy_ecs::prelude::*;

use crate::transition::TransitionError;
use crate::{Behavior, BehaviorIndex};

#[derive(Event)]
pub struct OnStart {
    pub index: BehaviorIndex,
}

#[derive(Event)]
pub struct OnPause {
    pub index: BehaviorIndex,
}

#[derive(Event)]
pub struct OnResume {
    pub index: BehaviorIndex,
}

#[derive(Event)]
pub struct OnActivate {
    pub index: BehaviorIndex,
    pub resume: bool,
}

#[derive(Event)]
pub struct OnStop<T: Behavior> {
    pub behavior: T,
}

#[derive(Event)]
pub struct OnError<T: Behavior>(pub TransitionError<T>);
