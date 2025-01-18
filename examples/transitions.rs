use std::time::Duration;

use bevy::prelude::*;
use moonshine_behavior::prelude::*;

fn main() {
    App::new().add_plugins((DefaultPlugins, bird_plugin)).run();
}

#[derive(Component, Debug, Reflect)]
#[require(Controller<Bird>)]
enum Bird {
    Idle { elapsed: Duration },
    Fly { duration: Duration },
    Chirp,
}

impl Behavior for Bird {
    /* ... */
}

fn bird_plugin(app: &mut App) {
    app.add_plugins(BehaviorPlugin::<Bird>::default())
        .add_systems(Startup, spawn_birds)
        .add_systems(
            Update,
            (bird_idle, bird_chirp, bird_fly, transition::<Bird>).chain(),
        );
}

fn spawn_birds(mut commands: Commands) {
    // Spawn a Bird with initial behavior Idle.
    commands.spawn(Bird::Idle {
        elapsed: Duration::ZERO,
    });

    // Spawn a Bird with initial behavior Idle, and then start flying!
    commands.spawn((
        Bird::Idle {
            elapsed: Duration::ZERO,
        },
        Controller::next(Bird::Fly {
            duration: Duration::from_secs(5),
        }),
    ));
}

// Idle birds chirp every 5 seconds:
fn bird_idle(time: Res<Time>, mut query: Query<(Entity, &mut Bird, &mut Controller<Bird>)>) {
    for (entity, mut bird, mut controller) in &mut query {
        let Bird::Idle { elapsed } = bird.as_mut() else {
            continue;
        };

        if controller.is_started() || controller.is_resumed() {
            info!("Bird {entity} is idle!");
        }

        *elapsed += time.delta();
        if *elapsed < Duration::from_secs(3) {
            continue;
        }

        // The transition will happen before the next update.
        let future_result = controller.try_start(Bird::Chirp);

        // You can either poll the result (mainly useful for diagnostics), or just forget about it!
        // Behavior transitions only fail if `Behavior::allows_next` returns false.
        future_result.forget();

        *elapsed = Duration::ZERO;
    }
}

fn bird_chirp(mut query: Query<(Entity, &Bird, &mut Controller<Bird>)>) {
    for (entity, bird, mut controller) in &mut query {
        let Bird::Chirp = bird else {
            continue;
        };

        if controller.is_started() {
            info!("Bird {entity} chirps!");
        }

        controller.stop();
    }
}

fn bird_fly(time: Res<Time>, mut query: Query<(Entity, &mut Bird, &mut Controller<Bird>)>) {
    for (entity, mut bird, mut controller) in &mut query {
        let Bird::Fly { duration } = bird.as_mut() else {
            continue;
        };

        if controller.is_started() {
            info!("Bird {entity} flies!");
        }

        *duration = duration.saturating_sub(time.delta());

        // Stop flying
        if duration.is_zero() {
            controller.stop();
            continue;
        }
    }
}
