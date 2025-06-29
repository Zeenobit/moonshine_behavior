use bevy_ecs::prelude::*;

use crate::transition::TransitionError;
use crate::Behavior;

#[derive(Event)]
pub struct OnStart {
    pub index: usize,
}

#[derive(Event)]
pub struct OnPause {
    pub index: usize,
}

#[derive(Event)]
pub struct OnResume {
    pub index: usize,
}

#[derive(Event)]
pub struct OnStop<T: Behavior> {
    pub behavior: T,
}

#[derive(Event)]
pub struct OnError<T: Behavior>(pub TransitionError<T>);
