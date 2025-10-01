use std::time::Duration;

use bevy::prelude::*;
use moonshine_behavior::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            BehaviorPlugin::<Signal>::default(),
            //Shape2dPlugin::default(),
        ))
        .add_systems(Startup, (setup, spawn_lights))
        .add_systems(
            Update,
            (update_signal, transition::<Signal>, update_lights).chain(),
        )
        .run();
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
enum Signal {
    Green,
    Yellow(Duration),
    Red,
}

impl Behavior for Signal {
    fn filter_next(&self, next: &Self) -> bool {
        use Signal::*;
        match_next! {
            self => next,
            Green => Yellow(..),
            Yellow(..) => Red,
        }
    }
}

const GREEN_COLOR: Color = Color::srgb(0.2, 1., 0.);
const YELLOW_COLOR: Color = Color::srgb(1., 0.8, 0.);
const RED_COLOR: Color = Color::srgb(1., 0.2, 0.);
const OFF_COLOR: Color = Color::srgb(0.1, 0.1, 0.1);

#[derive(Component)]
#[require(Transform, GlobalTransform)]
struct GreenLight;

#[derive(Component)]
#[require(Transform, GlobalTransform)]
struct YellowLight;

#[derive(Component)]
#[require(Transform, GlobalTransform)]
struct RedLight;

fn setup(mut configs: ResMut<GizmoConfigStore>, mut commands: Commands) {
    const HELP_TEXT: &str = "
    Press 'Space' to start/reset the Signal cycle!\n";

    commands.spawn(Camera2d);
    commands.spawn(Text::new(HELP_TEXT));
    commands.spawn(Signal::Green);

    let (config, ..) = configs.config_mut::<DefaultGizmoConfigGroup>();
    config.enabled = true;
    config.line.width = 3.;
}

fn spawn_lights(mut commands: Commands) {
    commands.spawn((Transform::from_xyz(-50., 0., 0.), GreenLight));
    commands.spawn((Transform::from_xyz(0., 0., 0.), YellowLight));
    commands.spawn((Transform::from_xyz(50., 0., 0.), RedLight));
}

fn update_signal(
    time: Res<Time>,
    key: Res<ButtonInput<KeyCode>>,
    mut signal: Single<BehaviorMut<Signal>>,
) {
    use Signal::*;

    match signal.current() {
        Green => {
            if key.just_pressed(KeyCode::Space) {
                signal.start(Yellow(Duration::from_secs(3)));
            }
        }
        Yellow(mut duration) => {
            duration = duration.saturating_sub(time.delta());
            **signal = Yellow(duration);
            if duration.is_zero() {
                signal.start(Red);
            }
        }
        Red => {
            if key.just_pressed(KeyCode::Space) {
                signal.reset();
            }
        }
    }
}

fn update_lights(
    mut gizmos: Gizmos,
    query: Query<BehaviorRef<Signal>>,
    green: Single<Entity, With<GreenLight>>,
    yellow: Single<Entity, With<YellowLight>>,
    red: Single<Entity, With<RedLight>>,
    transform: Query<&GlobalTransform>,
) {
    use Signal::*;

    let mut draw_circle = |entity: Entity, color: Color| {
        let transform = transform.get(entity).unwrap();
        gizmos.circle(transform.translation(), 20., color);
    };

    for behavior in query.iter() {
        // Rules for each light:
        // - Stay on if it's the current signal
        // - Dim if passed
        // - Turn off otherwise

        if matches!(*behavior, Green) {
            draw_circle(*green, GREEN_COLOR);
        } else if behavior.iter().any(|b| matches!(b, Green)) {
            draw_circle(*green, GREEN_COLOR.darker(0.6));
        } else {
            draw_circle(*green, OFF_COLOR);
        }

        if matches!(*behavior, Yellow(..)) {
            draw_circle(*yellow, YELLOW_COLOR);
        } else if behavior.iter().any(|b| matches!(b, Yellow(..))) {
            draw_circle(*yellow, YELLOW_COLOR.darker(0.6));
        } else {
            draw_circle(*yellow, OFF_COLOR);
        }

        if matches!(*behavior, Red) {
            draw_circle(*red, RED_COLOR);
        } else if behavior.iter().any(|b| matches!(b, Red)) {
            draw_circle(*red, RED_COLOR.darker(0.6));
        } else {
            draw_circle(*red, OFF_COLOR);
        }
    }
}
