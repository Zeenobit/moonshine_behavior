# 🎚️ Moonshine Behavior
Minimalistic state machine for [Bevy](https://github.com/bevyengine/bevy) game engine.

## Overview

This crates is designed to provide a simple, stack-based implementation of state machines for Bevy entities.

### Features
- Simple: Minimal overhead for defining and setting up behaviors.
- Behaviors can be started, paused, resumed, and stopped.
- Event driven API which allows systems to react to behavior changes on entities.
- Multiple behaviors with different types may exist on the same entity to define complex state machines.

## Usage

A behavior, typically implemented as an `enum`, is a `Component` which represents some state of its entity. Each behavior is associated with a stack.
When the next behavior is started, the current one is pushed onto the stack (if resumable) and paused.

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
# use bevy::prelude::*;
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

#### 2. Implement the `Behavior` trait:
```rust,ignore
impl Behavior for Bird {
    fn allows_next(&self, next: &Self) -> bool {
        use Bird::*;
        match self {
            Idle => matches!(next, Sleep | Fly | Chirp),
            Fly => matches!(next, Chirp),
            Sleep | Chirp => false,
        }
    }
}
```
This trait defines the possible transitions for your behavior.
In this example:
  - a bird may sleep, fly, or chirp when idle
  - a bird may chirp when flying
  - a bird may not do anything else when sleeping or chirping

#### 3. Register the `Behavior` and its transition:
Add a `BehaviorPlugin<T>` to your `App` to register the behavior events and types.
Use `transition()` system to trigger behavior transitions whenever you want.
```rust,ignore
app.add_plugins(BehaviorPlugin::<Bird>::default())
    .add_systems(Update, transition::<Bird>);
```

You can define your systems before or after the `transition` system.
Usually, systems that cause behavior change should run before transition while systems that handle behavior logic should run after transition.
  
#### 4. Spawn a `BehaviorBundle`:
For behavior system to work, you must insert your behavior using a `BehaviorBundle`.
This bundle also inserts an instance of your behavior. This is referred to as the **Initial Behavior**.

```rust,ignore
fn spawn_bird(mut commands: Commands) {
    commands.spawn(BehaviorBundle::<Bird>::default());
}
```

To spawn a bird with a specific initial behavior use `BehaviorBundle::<B>::new()`.

> ⚠️ **WARNING**</br>
> The initial behavior may never be stopped. Doing so would trigger an error.

### Transitions

TODO: Need up to date documentation.

See [transition.rs](examples/transitions.rs) for examples.

When a transition is requested, it is not invoked immediately. Instead, it is invoked whenever the registered `transition()` system is run.
You may register your systems before or after `transition()` to perform any logic as required.

> ⚠️ **WARNING**<br/>
> Be mindful that only one transition may be invoked per application update, per entity. This is an intentional design choice.
> If multiple transitions are requested on the same entity within the same update cycle, only the last one is invoked, and a warning is logged.

### Events

Any time a transition is invoked, an associated event is dispatched. These events may be used by other systems to react to behavior changes.

Each event (except `StoppedEvent`) carries only the entity ID for which the behavior was started, paused, or resumed. `StoppedEvent` carries the entity ID in additional to the stopped behavior data.

For `StartedEvent` and `ResumedEvent`, the behavior exists on the entity itself.
You may access it either using a normal query (e.g. `Query<&Bird>`), or using `BehaviorRef`.
```rust,ignore
fn on_chirp_started(mut events: Started<Bird>, query: Query<BehaviorRef<Bird>>) {
    for event in events.iter() {
        let entity = event.entity();
        let behavior = query.get(entity).unwrap();
        if let Chirp = *behavior {
            info!("{entity:?} has started chirping!");
        }
    }
}

fn on_chirp_resumed(mut events: Resumed<Bird>, query: Query<BehaviorRef<Bird>>) {
    for event in events.iter() {
        let entity = event.entity();
        let behavior = query.get(entity).unwrap();
        if let Chirp = *behavior {
            info!("{entity:?} is chirping again!");
        }
    }
}
```
For `PausedEvent`, the paused behavior is the previous behavior on the data, which is accessible using `.previous()`:
```rust,ignore
fn on_chirp_paused(mut events: Paused<Bird>, query: Query<BehaviorRef<Bird>>) {
    for event in events.iter() {
        let entity = event.entity();
        let behavior = query.get(entity).unwrap();
        if let Chirp = behavior.previous() {
            info!("{entity:?} is no longer chirping.");
        }
    }
}
```
For `StoppedEvent`, the stopped behavior is accessible through the event itself:
```rust,ignore
fn on_chirp_stopped(mut events: Stopped<Bird>) {
    for event in events.iter() {
        let entity = event.entity();
        let behavior = event.behavior();
        if let Chirp = *behavior {
            info!("{entity:?} has stopped chirping.");
        }
    }
}
```
### Activation/Suspension

In some cases, it may be necessary to run some logic if a behavior is paused OR stopped (suspension), or started OR resumed (activation).<br/>
To handle activation and suspension, you may use a standard [`Changed`](https://docs.rs/bevy/latest/bevy/ecs/query/struct.Changed.html) query:
```rust,ignore
fn on_chirp_activated(query: Query<BehaviorRef<Bird>, Changed<Bird>>) {
    if let Ok(behavior) = query.get_single() {
        if let Chirp = *behavior {
            info!("{entity:?} is chirping!");
        }        
    }
}

fn on_chirp_suspended(query: Query<BehaviorRef<Bird>, Changed<Bird>>) {
    if let Ok(behavior) = query.get_single() {
        if let Chirp = behavior.previous() {
            info!("{entity:?} is not chirping.");
        }        
    }
}
```

## Examples

See [bird.rs](examples/bird.rs) for a complete implementation of the `Bird` behavior.

## Support

Find me on Bevy Discord server, or post an issue.
