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
```rust
use bevy::prelude::*;

#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
enum Bird {
    #[default]
    Idle,
    Fly,
    Sleep,
    Chirp,
}
```

Behaviors are often implemented as an `enum` since they represent a finite set of states. This is not a hard requirement. Any `struct` may be used to represent behavior data as well, such as:
```rust
# use bevy::prelude::*;
#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
struct Bird {
    flying: bool,
    sleeping: bool,
    chirping: bool,
}
```

You may even use nested enums or structs to represent complex state machines:
```rust
use bevy::prelude::*;

#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
enum Bird {
    #[default]
    Idle,
    Fly(Fly),
    Sleep(Sleep),
    Chirp(Chirp),
}

#[derive(Default, Debug, Reflect)]
enum Fly {
    #[default]
    Normal,
    Hunt,
    Flee,
}

#[derive(Default, Debug, Reflect)]
struct Sleep {
    duration: f32,
}

#[derive(Default, Debug, Reflect)]
struct Chirp {
    count: usize,
}
```

#### 2. Implement the [`Behavior`] trait:

```rust,ignore
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
```rust,ignore
app.add_plugins(BehaviorPlugin::<Bird>::default())
    .add_systems(Update, transition::<Bird>);
```

You may define your systems before or after the `transition` system.

Usually, systems that cause behavior change should run before transition while systems that handle behavior logic should run after transition. However, this is not a strict requirement. Just be mindful of frame delays!
  
#### 4. Spawn

The first instance of `T` which is inserted into the entity is considered the **Initial Behavior**:
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
        TransitionSequence::new()
            .then_wait_for(Bird::Chirp)
            .then(Bird::Fly)
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
- [`try_start`] Attempts to start a new behavior if there is currently no pending transition and the current behavior allows it.
- [`interrupt_start`] Stops all behaviors which [yield][`filter_yield`] to the new behavior, and then starts the new behavior.
- [`stop`] Stops the current behavior and resumes the previous one.
- [`reset`] Stops all behaviors and resets the entity to its initial state.

Regardless of the method used, all transition may fail if:
- The new behavior does not allow the new behavior to start at the exact time of [`transition`]. See [`filter_next`].
- The current behavior is the initial behavior and a stop is requested. The initial behavior may never be stopped.

To completely stop the behavior, including the initial, you must remove the entire behavior from the entity.
To do this, use [`remove_with_require::<T>()`](https://docs.rs/bevy/latest/bevy/ecs/prelude/struct.EntityCommands.html#method.remove_with_requires) to remove the initial behavior and the entire behavior stack.

### Events

Any time a transition is invoked, a [`BehaviorEvent`] is sent. These events may be used by other systems to react to behavior changes.

See [documentation][`BehaviorEvent`] for complete details and usage examples.

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
[`BehaviorRef<T>`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorRef.html
[`BehaviorMut<T>`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMut.html
[`start`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMut.html#method.start
[`try_start`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMut.html#method.try_start
[`interrupt_start`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMut.html#method.interrupt_start
[`stop`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMut.html#method.stop
[`reset`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/struct.BehaviorMut.html#method.reset
[`filter_yield`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/trait.Behavior.html#method.filter_yield
[`filter_next`]:https://docs.rs/moonshine-behavior/latest/moonshine_behavior/trait.Behavior.html#method.filter_next