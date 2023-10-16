# 🎚️ Moonshine Behavior
Minimalistic state machine for [Bevy](https://github.com/bevyengine/bevy) game engine.

## Overview

This crates is designed to provide a simple, stack-based, lightweight implementation of state machines for Bevy entities.

### Features
- Simple. Minimal overhead for defining and setting up behaviors.
- Behaviors can be started, paused, resumed, and stopped.
- Event driven API which allows systems to react to behavior changes on entities.
- Multiple behaviors with different types may exist on the same entity to define complex state machines.

## Usage

A behavior, typically implemented as an `enum`, is a `Component` which represents some state of its entity. Each behavior is associated with a stack.
When the next behavior is started, the current one is pushed onto the stack (if resumable) and paused.

### Setup

#### 1. Define your behavior data as an `enum`:
```rust
use bevy::prelude::*;
use moonshine_behavior::prelude::*;

#[derive(Component, Default, Debug, Reflect, FromReflect)]
#[reflect(Component)]
enum Bird {
    #[default]
    Idle,
    Fly,
    Sleep,
    Chirp,
}
```

#### 2. Implement the `Behavior` trait:
```rust
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
  - a bird may not fly or chirp when sleeping

#### 3. Register the `Behavior` and its transition:
Add a `BehaviorPlugin<T>` to your `App` to register the behavior events and types.
Use `transition()` system to trigger behavior transitions whenever you want.
```rust
app.add_plugins(BehaviorPlugin::<Bird>::default())
    .add_systems(Update, transition::<Bird>);
```
  
#### 4. Spawn a `BehaviorBundle`:
For behavior system to work, you must insert your behavior using a `BehaviorBundle`.
This bundle also inserts an instance of your behavior. This is referred to as the initial behavior.
```rust
fn spawn_bird(mut commands: Commands) {
    commands.spawn(BehaviorBundle::<Bird>::default());
}
```

### Transitions

An entity spawned with a `BehaviorBundle` may be queried using `BehaviorRef` and `BehaviorMut` world queries.

- `BehaviorRef` may be used to read the current/previous behaviors.
- `BehaviorMut` may be used to read the current/previous behaviors and request behavior transitions.

To access current behavior, use `Deref` on either `BehaviorRef` or `BehaviorMut`.<br/>
To access previous behavior, use `.previous()`:
```rust
fn is_chirping_while_flying(bird: Query<BehaviorRef<Bird>>) -> bool {
    let behavior = bird.single();
    matches!(*behavior, Chirp) && matches!(behavior.previous(), Some(Fly))
}
```

To start some next behavior, use `.try_start()`:
```rust
fn chirp(mut bird: Query<BehaviorMut<Bird>>) {
    bird.single_mut().try_start(Chirp);
}
```

To stop current behavior and resume the previous behavior, use `.stop()`:
```rust
fn stop(mut bird: Query<BehaviorMut<Bird>>) {
    bird.single_mut().stop();
}
```

To stop current behavior and resume the initial behavior, use `.reset()`:
```rust
fn reset(mut bird: Query<BehaviorMut<Bird>>) {
    bird.single_mut().reset();
}
```

When a transition is requested, it is not invoked immediately. Instead, it is invoked whenever the registered `transition()` system is run.
You may register your systems before or after `transition()` to perform any logic as required.

> **Warning**<br/>
> Be mindful that only one transition may be invoked per application update, per entity. This is an intentional design choice.<br/>
> If multiple transitions are requested on the same entity within the same update cycle, only the last one is invoked, and a warning is logged.

### Events

Any time a transition is invoked, an associated event is dispatched. These events may be used by other systems to react to behavior changes.

Each event (except `StoppedEvent`) carries only the entity ID for which the behavior was started, paused, or resumed.

For `StartedEvent` and `ResumedEvent`, the behavior exists on the entity itself.
You may access it either using a normal query (e.g. `Query<&Bird>`), or using `BehaviorRef`.
```rust
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
```rust
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
```rust
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
To handle activation and suspension, you may use a simple `Changed` query:
```rust
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
