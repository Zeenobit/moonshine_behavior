use std::marker::PhantomData;

use bevy_app::prelude::*;
use bevy_reflect::{prelude::*, GetTypeRegistration, Typed};

use crate::{Behavior, Memory, Transition, TransitionSequence};

pub struct BehaviorPlugin<T: Behavior>(PhantomData<T>);

impl<T: Behavior> Default for BehaviorPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: RegisterableBehavior> Plugin for BehaviorPlugin<T> {
    fn build(&self, app: &mut App) {
        app.register_type::<Transition<T>>()
            .register_type::<Memory<T>>()
            .register_type::<TransitionSequence<T>>()
            .register_required_components::<T, Transition<T>>();
    }
}

pub trait RegisterableBehavior: Behavior + FromReflect + GetTypeRegistration + Typed {}

impl<T: Behavior + FromReflect + GetTypeRegistration + Typed> RegisterableBehavior for T {}
