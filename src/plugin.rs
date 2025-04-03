use std::marker::PhantomData;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::{prelude::*, GetTypeRegistration, Typed};

use crate::events::BehaviorEventsPlugin;
use crate::{sequence::Sequence, Behavior, Memory, Transition};

pub struct BehaviorPlugin<T: Behavior>(PhantomData<T>);

impl<T: Behavior> Default for BehaviorPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: RegisterableBehavior + Component> Plugin for BehaviorPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_plugins(BehaviorEventsPlugin::<T>::default())
            .register_type::<Transition<T>>()
            .register_type::<Memory<T>>()
            .register_type::<Sequence<T>>()
            .register_required_components::<T, Transition<T>>();
    }
}

pub trait RegisterableBehavior: Behavior + FromReflect + GetTypeRegistration + Typed {}

impl<T: Behavior + FromReflect + GetTypeRegistration + Typed> RegisterableBehavior for T {}
