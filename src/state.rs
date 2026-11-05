//! A [`BehaviorState`] is some data and logic which may be shared by multiple [`Behavior`] types.
//!
//! Different behaviors often have some states in common. For example, a bird, a cat, and a dog may
//! all sleep in the same way, even though only a dog can bark.
//!
//! To avoid repeating this logic for each behavior, define the state as its own type, and write
//! the systems and observers for it generically over any [`BehaviorState`] of that type.
//!
//! # Example
//!
//! ```rust
//! use std::time::Duration;
//!
//! use bevy::prelude::*;
//! use moonshine_behavior::prelude::*;
//!
//! // Common state:
//! #[derive(Debug, Reflect)]
//! struct Sleep(Duration);
//!
//! // Common logic:
//! fn sleep_plugin<T: BehaviorState<Sleep>>(app: &mut App) {
//!     app.add_observer(start_sleep::<T>);
//! }
//!
//! fn start_sleep<T: BehaviorState<Sleep>>(event: OnStart<T>, query: Query<BehaviorRef<T>>) {
//!     let Ok(behavior) = query.get(event.instance.entity()) else {
//!         return;
//!     };
//!
//!     let Some(Sleep(duration)) = behavior[event.index].state() else {
//!         return;
//!     };
//!
//!     info!("Sleeping for {duration:?}");
//! }
//!
//! // Behaviors with the common state:
//! #[derive(Component, Debug, Reflect)]
//! #[reflect(Component)]
//! enum Cat {
//!     Idle,
//!     Sleep(Sleep),
//!     Purr,
//! }
//!
//! impl Behavior for Cat {}
//!
//! impl_behavior_state!(Cat::Sleep as Sleep);
//!
//! #[derive(Component, Debug, Reflect)]
//! #[reflect(Component)]
//! enum Dog {
//!     Idle,
//!     Sleep(Sleep),
//!     Bark,
//! }
//!
//! impl Behavior for Dog {}
//!
//! impl_behavior_state!(Dog::Sleep as Sleep);
//!
//! App::new()
//!     .add_plugins((BehaviorPlugin::<Cat>::default(), sleep_plugin::<Cat>))
//!     .add_plugins((BehaviorPlugin::<Dog>::default(), sleep_plugin::<Dog>))
//!     .add_systems(Update, (transition::<Cat>, transition::<Dog>));
//! ```
//!
//! See [pets.rs](https://github.com/Zeenobit/moonshine_behavior/blob/main/examples/pets.rs) for a
//! complete example.
//!
//! # Limitations
//!
//! ## Type Annotations
//!
//! A behavior may implement [`BehaviorState`] for many types. If you are not using the state from
//! generic code, where `T: BehaviorState<Sleep>` is the only bound, then the compiler needs to be
//! told which state you are referring to:
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use moonshine_behavior::prelude::*;
//! # #[derive(Debug)]
//! # struct Sleep;
//! # #[derive(Debug)]
//! # struct Eat;
//! # #[derive(Component, Debug)]
//! # enum Dog {
//! #     Sleep(Sleep),
//! #     Eat(Eat),
//! # }
//! # impl Behavior for Dog {}
//! # impl_behavior_state!(Dog::Sleep as Sleep);
//! # impl_behavior_state!(Dog::Eat as Eat);
//! fn is_asleep(dog: &Dog) -> bool {
//!     let sleep: Option<&Sleep> = dog.state();
//!     sleep.is_some()
//! }
//!
//! fn is_eating(dog: &Dog) -> bool {
//!     BehaviorState::<Eat>::is_in_state(dog)
//! }
//! ```
//!
//! ## Change Detection
//!
//! When used through a [`BehaviorMut`], [`state_mut`](BehaviorState::state_mut) always flags the
//! behavior as changed, even if it returns [`None`], because it requires mutable access to the behavior.
//!
//! If change detection is important, check if the behavior is in the state first:
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use moonshine_behavior::prelude::*;
//! # #[derive(Debug)]
//! # struct Sleep(f32);
//! fn sleep<T: BehaviorState<Sleep>>(mut query: Query<BehaviorMut<T>>) {
//!     for mut behavior in &mut query {
//!         if !behavior.is_in_state() {
//!             continue;
//!         }
//!
//!         let Some(Sleep(duration)) = behavior.state_mut() else {
//!             // SAFE: Already checked above.
//!             unreachable!();
//!         };
//!
//!         *duration -= 1.;
//!     }
//! }
//! ```
//!
//! # Sharing a State between Variants
//!
//! Multiple states of a behavior may use the same type. In this case, [`state`](BehaviorState::state)
//! may return the same type for any of them, so the common logic applies to all of them.
//!
//! [`impl_behavior_state`] only maps a single variant, so for now, this requires a manual implementation:
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use moonshine_behavior::prelude::*;
//! #[derive(Debug)]
//! struct Move {
//!     speed: f32,
//! }
//!
//! #[derive(Component, Debug)]
//! enum Human {
//!     Idle,
//!     Walk(Move),
//!     Run(Move),
//! }
//!
//! impl Behavior for Human {}
//!
//! impl BehaviorState<Move> for Human {
//!     fn state(&self) -> Option<&Move> {
//!         match self {
//!             Human::Walk(state) | Human::Run(state) => Some(state),
//!             _ => None,
//!         }
//!     }
//!
//!     fn state_mut(&mut self) -> Option<&mut Move> {
//!         match self {
//!             Human::Walk(state) | Human::Run(state) => Some(state),
//!             _ => None,
//!         }
//!     }
//! }
//! ```
//!
//! [`BehaviorMut`]: crate::BehaviorMut
//! [`OnStart`]: crate::events::OnStart
//! [`OnPause`]: crate::events::OnPause
//! [`OnResume`]: crate::events::OnResume
//! [`OnActivate`]: crate::events::OnActivate
//! [`OnStop`]: crate::events::OnStop
//! [`impl_behavior_state`]: crate::impl_behavior_state

use crate::Behavior;

/// A [`Behavior`] which may be in a state of type `T`.
///
/// You may implement this for a behavior using [`impl_behavior_state`](crate::impl_behavior_state),
/// or manually if the behavior is not an `enum`.
///
/// See [module documentation](self) for details and usage.
pub trait BehaviorState<T>: Behavior {
    /// Returns a reference to the state T if it is the behavior's current state.
    fn state(&self) -> Option<&T>;

    /// Returns a mutable reference to the state T if it is the behavior's current state.
    ///
    /// See [module documentation](self#change-detection) for details on change detection.
    fn state_mut(&mut self) -> Option<&mut T>;

    /// Returns `true` if this behavior is in the state T.
    fn is_in_state(&self) -> bool {
        self.state().is_some()
    }
}

/// Implements [`BehaviorState`] for a single variant of an `enum` [`Behavior`](crate::Behavior).
///
/// This macro is **experimental** and only supports `enum` behaviors for now.
/// Behaviors which are not an `enum` must implement [`BehaviorState`](crate::state::BehaviorState) manually.
///
/// # Usage
///
/// The variant must contain exactly one field, which is the state.
/// The name of the behavior must be given as a plain identifier, without any path or generics.
///
/// Use `Behavior::Variant as State` for a tuple variant,
/// and `Behavior::Variant { field } as State` for a variant with a named field.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use moonshine_behavior::{impl_behavior_state, Behavior};
///
/// #[derive(Debug)]
/// struct Sleep;
///
/// #[derive(Debug)]
/// struct Eat;
///
/// #[derive(Component, Debug)]
/// enum Dog {
///     Idle,
///     Sleep(Sleep),
///     Eat { eat: Eat },
/// }
///
/// impl Behavior for Dog {}
///
/// impl_behavior_state!(Dog::Sleep as Sleep);
/// impl_behavior_state!(Dog::Eat { eat } as Eat);
/// ```
#[macro_export]
macro_rules! impl_behavior_state {
    // Tuple variants
    ($t:ident::$v:ident as $s:ty) => {
        impl $crate::state::BehaviorState<$s> for $t {
            fn state(&self) -> Option<&$s> {
                if let $t::$v(state) = self {
                    return Some(state);
                };
                None
            }

            fn state_mut(&mut self) -> Option<&mut $s> {
                if let $t::$v(state) = self {
                    return Some(state);
                };
                None
            }
        }
    };

    // Named fields
    ($t:ident::$v:ident { $f:ident } as $s:ty) => {
        impl $crate::state::BehaviorState<$s> for $t {
            fn state(&self) -> Option<&$s> {
                if let $t::$v { $f } = self {
                    return Some($f);
                };
                None
            }

            fn state_mut(&mut self) -> Option<&mut $s> {
                if let $t::$v { $f } = self {
                    return Some($f);
                };
                None
            }
        }
    };
}
