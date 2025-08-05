use std::marker::PhantomData;

use bevy_app::prelude::*;
use bevy_reflect::{prelude::*, GetTypeRegistration, Typed};

use crate::{Behavior, Memory, Transition, TransitionQueue};

/// A [`Plugin`] for any [`Behavior`] type.
///
/// This plugin must be added to the [`App`] for behavior [`Transitions`](Transition) to work correctly.
/// You must also add the [`transition`](crate::transition::transition) system separately somewhere in your schedule.
///
/// # Example
/// ```rust
/// use bevy::prelude::*;
/// use moonshine_behavior::prelude::*;
///
/// #[derive(Component, Debug, Reflect)]
/// #[reflect(Component)]
/// struct B;
///
/// impl Behavior for B {}
///
/// fn main() {
///     App::new()
///         /* ... */
///         .add_plugins(BehaviorPlugin::<B>::default())
///         .add_systems(Update, transition::<B>)
///         /* ... */;
/// }
/// ```
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
            .register_type::<TransitionQueue<T>>()
            .register_required_components::<T, Transition<T>>();
    }
}

#[doc(hidden)]
pub trait RegisterableBehavior: Behavior + FromReflect + GetTypeRegistration + Typed {}

impl<T: Behavior + FromReflect + GetTypeRegistration + Typed> RegisterableBehavior for T {}
