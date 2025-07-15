# 🎚️ Moonshine Behavior

[![crates.io](https://img.shields.io/crates/v/moonshine-behavior)](https://crates.io/crates/moonshine-behavior)
[![downloads](https://img.shields.io/crates/dr/moonshine-behavior?label=downloads)](https://crates.io/crates/moonshine-behavior)
[![docs.rs](https://docs.rs/moonshine-behavior/badge.svg)](https://docs.rs/moonshine-behavior)
[![license](https://img.shields.io/crates/l/moonshine-behavior)](https://github.com/Zeenobit/moonshine_behavior/blob/main/LICENSE)
[![stars](https://img.shields.io/github/stars/Zeenobit/moonshine_behavior)](https://github.com/Zeenobit/moonshine_behavior)

Minimalistic state machine for [Bevy](https://github.com/bevyengine/bevy) entities.

## Overview

This crates is designed to provide a simple, stack-based implementation of state machines for Bevy entities.

### Features

- Simple: Minimal overhead for defining and setting up behaviors.
- Behaviors can be started, paused, resumed, and stopped.
- Event driven API which allows systems to react to behavior state change per entity.
- Multiple behaviors with different types may exist on the same entity to define complex state machines.

## Usage

A behavior, typically implemented as an `enum`, is a [`Component`] which represents some state of its entity. Each behavior is associated with a **Stack**.

When a new behavior starts, the current state is removed from the entity and pushed onto the stack (if resumable) and paused.

When a behavior stops, the previous state is removed from the stack and inserted back into the entity.

### Setup

#### 1. Define your behavior data as a [`Component`](https://docs.rs/bevy/latest/bevy/ecs/component/trait.Component.html)

Behaviors are often implemented as an `enum` since they are ideal for representing finite state machines. However, this is not a strict requirement. Any `struct` may also be used to represent behavior states.

```rust
use bevy::prelude::*;

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
enum Bird {
    /* ... */
}
```

#### 2. Implement the [`Behavior`] trait:

```rust
use bevy::prelude::*;
use moonshine_behavior::prelude::*;

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
enum Bird {
    Idle,
    Fly,
    Sleep,
    Chirp,
}

impl Behavior for Bird {
    fn filter_next(&self, next: &Self) -> bool {
        use Bird::*;
        match_next! {
            self => next,
            Idle => Sleep | Fly | Chirp,
            Fly => Chirp,
        }
    }
}
```

This trait defines the possible transitions for your behavior.

In this example:
  - a bird may sleep, fly, or chirp when idle
  - a bird may chirp when flying
  - a bird may not do anything else when sleeping or chirping

This trait has additional methods for more advanced usage. See [`Behavior`] trait documentation for full details.

#### 3. Register the `Behavior` and its transition:
Add a [`BehaviorPlugin`] and the [`transition`] system to your [`App`] to trigger behavior transitions whenever you want.
```rust
use bevy::prelude::*;
use moonshine_behavior::prelude::*;

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
enum Bird {
    /* ... */
}

impl Behavior for Bird {
    /* ... */
}

app.add_plugins(BehaviorPlugin::<Bird>::default())
    .add_systems(Update, transition::<Bird>);
```

You may define your systems before or after the `transition` system.

Usually, systems that cause behavior change should run before transition while systems that handle behavior logic should run after transition. However, this is not a strict requirement. Just be mindful of frame delays!
  
#### 4. Spawn

The first instance of `T` which is inserted into the entity is defined as the **Initial Behavior**:
```rust,ignore
fn spawn_bird(mut commands: Commands) {
    commands.spawn(Bird::Idle); // <--- Bird starts in Idle state
}
```

You may also spawn a bird with a [`Transition`]:
```rust,ignore
fn spawn_bird(mut commands: Commands) {
    commands.spawn((Bird::Idle, Next(Bird::Chirp))); // <--- Bird starts in Idle state and then Chirps!
}
```

Or maybe even a [`TransitionSequence`]:

```rust,ignore
fn spawn_bird(mut commands: Commands) {
    commands.spawn((
        Bird::Idle,
        TransitionSequence::wait_for(Bird::Chirp).then(Bird::Fly)
    ));
}
```

#### 5. Query

To manage the behavior of an entity, you may use the [`BehaviorRef<T>`] and [`BehaviorMut<T>`] [`Query`] terms:

```rust,ignore
fn update_bird(mut query: Query<BehaviorMut<Bird>>) {
    for mut behavior in query.iter_mut() {
        match behavior.current() {
            Bird::Idle => {
                // Do something when the bird is idle
            }
            Bird::Chirp => {
                // TODO: Play Chirp sound!
                behavior.stop(); // <-- Go back to the previous state
            }
            _ => { /* ... */ }
        }
    }
}
```

`BehaviorRef<T>` is a read-only reference to the current behavior and the entire stack.

`BehaviorMut<T>` extends `BehaviorRef<T>` and allows you to modify the behavior as well.

Note that you may also access your behavior as `&T` or `&mut T` directly (it is just a component after all!).

`Transition<T>` is also directly accessible as a component, however it is more ergonomic to use `BehaviorMut<T>` for transitions.

### Transitions

When a transition is requested, it is not invoked immediately. Instead, it is invoked whenever the registered [`transition`] system is run.
You may register your systems before or after `transition::<T>` to perform any logic as required.

> ⚠️ **WARNING**<br/>
> In most cases, only one transition is allowed per entity, per cycle.
>
> This is by design to allow each state to get at least one active frame.
>
> The exception to this is during an interruption or a reset, where multiple behaviors may be stopped at once.

To invoke a transition, you may use the [`BehaviorMut<T>`]. There are several methods for invoking transitions:

- [`start`] Pauses the current behavior and starts a new one.
- [`interrupt_start`] Stops all behaviors which [yield][`filter_yield`] to the new behavior, and then starts the new behavior.
- [`interrupt_resume`] Stops all behaviors above a given index in the stack and resumes it.
- [`interrupt_stop`] Stops all behaviors above and including given index in the stack, resuming the previous one.
- [`stop`] Stops the current behavior and resumes the previous one.
- [`reset`] Stops all behaviors and resets the entity to its initial state.

Regardless of the method used, all transition may fail if:
- The new behavior does not allow the new behavior to start at the exact time of [`transition`]. See [`filter_next`].
- The current behavior is the initial behavior and a stop is requested. The initial behavior may never be stopped.

To completely stop the behavior, including the initial, you must remove the entire behavior from the entity.
To do this, use [`remove_with_require::<T>()`](https://docs.rs/bevy/latest/bevy/ecs/prelude/struct.EntityCommands.html#method.remove_with_requires) to remove the initial behavior and the entire behavior stack.

### Events

When a transition is invoked, several behavior [`events`] are triggered.
You may use the following triggers to react to these events:

- [`Trigger<OnStart, T>`] - Triggered when a new behavior starts.
- [`Trigger<OnPause, T>`] - Triggered when a behavior is paused as the next one starts.
- [`Trigger<OnResume, T>`] - Triggered when a behavior is resumed as the previous one stops.
- [`Trigger<OnStop<T>, T>`] - Triggered when a behavior stops (⚠️ Note the additional type parameter).
- [`Trigger<OnActivate, T>`] - Triggered when a behavior is activated (started OR resumed).

See [`events`] documentation for more details.

### Hooks

In addition to events, you may also use hooks to perform immediate actions during a [`transition`]. Hooks are methods on the [`Behavior`] trait which may optionally be implemented by you:

```rust,ignore
impl Behavior for Bird {
    fn on_start(&self, _previous: Option<&Self>, mut commands: InstanceCommands<Self>) {
        match self {
            Bird::Chirp => {
                commands.insert(PlayAudio { /* ... */});
            }
            _ => { /* ... */ }
        }
    }
}
```

These hook commands would be executed immediately after [`transition`] is invoked. They are mainly useful when trying to minimize frame delays between state changes.

## Examples

See [signal.rs](examples/signal.rs) for a complete example.

## Support

Please [post an issue](https://github.com/Zeenobit/moonshine_behavior/issues/new) for any bugs, questions, or suggestions.

You may also contact me on the official [Bevy Discord](https://discord.gg/bevy) server as **@Zeenobit**.

[`Entity`]:https://docs.rs/bevy/latest/bevy/ecs/entity/struct.Entity.html
[`Component`]:https://docs.rs/bevy/latest/bevy/ecs/component/trait.Component.html
[`App`]:https://docs.rs/bevy/latest/bevy/app/struct.App.html
[`Query`]:https://docs.rs/bevy/latest/bevy/ecs/system/struct.Query.html
[`Behavior`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/trait.Behavior.html
[`BehaviorPlugin`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorPlugin.html
[`transition`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/fn.transition.html
[`Transition`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.Transition.html
[`TransitionSequence`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.TransitionSequence.html
[`BehaviorEvent`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorEvent.html
[`events`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/events/index.html
[`Trigger<OnStart, T>`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/events/struct.OnStart.html
[`Trigger<OnStop<T>, T>`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/events/struct.OnStop.html
[`Trigger<OnPause, T>`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/events/struct.OnReset.html
[`Trigger<OnResume, T>`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/events/struct.OnResume.html
[`Trigger<OnActivate, T>`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/events/struct.OnActivate.html
[`BehaviorRef<T>`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorRef.html
[`BehaviorMut<T>`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMut.html
[`start`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMutItem.html#method.start
[`interrupt_start`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMutItem.html#method.interrupt_start
[`interrupt_resume`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMutItem.html#method.interrupt_resume
[`interrupt_stop`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMutItem.html#method.interrupt_stop
[`stop`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMutItem.html#method.stop
[`reset`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMutItem.html#method.reset
[`filter_yield`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/trait.Behavior.html#method.filter_yield
[`filter_next`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/trait.Behavior.html#method.filter_next