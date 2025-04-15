use std::time::Duration;

use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use moonshine_behavior::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            BehaviorPlugin::<Signal>::default(),
            Shape2dPlugin::default(),
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
        match (self, next) {
            (Green, Yellow(..)) | (Yellow(..), Red) => true,
            _ => false,
        }
    }
}

const GREEN_COLOR: Color = Color::srgb(0.2, 1., 0.);
const YELLOW_COLOR: Color = Color::srgb(1., 0.8, 0.);
const RED_COLOR: Color = Color::srgb(1., 0.2, 0.);
const OFF_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);

#[derive(Component)]
struct GreenLight;

#[derive(Component)]
struct YellowLight;

#[derive(Component)]
struct RedLight;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn(Signal::Green);
}

fn spawn_lights(mut commands: Commands) {
    let mut config = ShapeConfig::default_2d();
    config.color = OFF_COLOR;
    config.set_translation((-50., 0., 0.).into());
    commands.spawn((ShapeBundle::circle(&config, 20.), GreenLight));
    config.set_translation((0., 0., 0.).into());
    commands.spawn((ShapeBundle::circle(&config, 20.), YellowLight));
    config.set_translation((50., 0., 0.).into());
    commands.spawn((ShapeBundle::circle(&config, 20.), RedLight));
}

fn update_signal(
    time: Res<Time>,
    key: Res<ButtonInput<KeyCode>>,
    mut query: Query<BehaviorMut<Signal>>,
) {
    use Signal::*;

    let mut signal = query.single_mut();
    match *signal {
        Green => {
            if key.just_pressed(KeyCode::Space) {
                signal.start(Yellow(Duration::from_secs(3)));
            }
        }
        Yellow(mut duration) => {
            duration = duration.saturating_sub(time.delta());
            *signal = Yellow(duration);
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
    mut events: TransitionEvents<Signal>,
    behavior: Query<BehaviorRef<Signal>>,
    green: Single<Entity, With<GreenLight>>,
    yellow: Single<Entity, With<YellowLight>>,
    red: Single<Entity, With<RedLight>>,
    mut fill: Query<&mut ShapeFill>,
) {
    use Signal::*;

    for event in events.read() {
        use TransitionEvent::*;
        match event {
            Start { instance, .. } => {
                let behavior = behavior.get(instance.entity()).unwrap();
                match behavior.current() {
                    Green => fill.get_mut(*green).unwrap().color = GREEN_COLOR,
                    Yellow(_) => fill.get_mut(*yellow).unwrap().color = YELLOW_COLOR,
                    Red => fill.get_mut(*red).unwrap().color = RED_COLOR,
                };
            }
            Pause { instance, .. } => {
                let behavior = behavior.get(instance.entity()).unwrap();
                match behavior.previous().unwrap() {
                    Green => fill.get_mut(*green).unwrap().color = GREEN_COLOR.darker(0.6),
                    Yellow(_) => fill.get_mut(*yellow).unwrap().color = YELLOW_COLOR.darker(0.6),
                    Red => fill.get_mut(*red).unwrap().color = RED_COLOR.darker(0.6),
                }
            }
            Resume { instance, .. } => {
                let behavior = behavior.get(instance.entity()).unwrap();
                match behavior.current() {
                    Green => fill.get_mut(*green).unwrap().color = GREEN_COLOR,
                    Yellow(_) => fill.get_mut(*yellow).unwrap().color = YELLOW_COLOR,
                    Red => fill.get_mut(*red).unwrap().color = RED_COLOR,
                };
            }
            Stop { behavior, .. } => match behavior {
                Green => fill.get_mut(*green).unwrap().color = OFF_COLOR,
                Yellow(_) => fill.get_mut(*yellow).unwrap().color = OFF_COLOR,
                Red => fill.get_mut(*red).unwrap().color = OFF_COLOR,
            },
            Error { error, .. } => {
                error!("{error:?}");
            }
        }
    }
}
