use bevy::prelude::*;
use moonshine_behavior::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BehaviorPlugin::<Player>::default()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                transition::<Player>,
                (player_start, player_pause, player_resume, player_stop),
            )
                .chain(),
        )
        .run();
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
enum Player {
    Idle,
    Walk,
    Sing,
    Rest,
}

impl Behavior for Player {
    fn allows_next(&self, next: &Self) -> bool {
        use Player::*;
        match (self, next) {
            (Idle, Walk) | (Idle, Rest) | (Idle, Sing) => true,
            (Walk, Sing) => true,
            _ => false,
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Player::Idle);
    commands.spawn(Camera2d);
    commands.spawn(Text2d::new(
        "Hello, Player! :)\n
Press W to Walk\n
Press S to Sing\n
Press R to Rest\n
Press Backspace to Stop current activity.\n
Press X to Reset\n\n
Look at console for output!",
    ));
}

fn player_start(mut events: StartEvents<Player>, query: Query<BehaviorRef<Player>>) {
    for Start { instance } in events.read() {
        use Player::*;
        let behavior = query.get(instance.entity()).unwrap();
        match *behavior {
            Idle => info!("Player is idle."),
            Walk => info!("Player has started walking ..."),
            Sing => match behavior.previous().unwrap() {
                Idle => info!("Player is singing!"),
                Walk => info!("Player is walking and singing!"),
                _ => unreachable!(),
            },
            Rest => info!("Player is resting ..."),
        }
    }
}

fn player_pause(mut events: PauseEvents<Player>, query: Query<BehaviorRef<Player>>) {
    for Pause { instance } in events.read() {
        use Player::*;
        let behavior = query.get(instance.entity()).unwrap();
        match behavior.previous().unwrap() {
            Idle => info!("Player is no longer idle."),
            _ => (),
        }
    }
}

fn player_resume(mut events: ResumeEvents<Player>, query: Query<BehaviorRef<Player>>) {
    for Resume { instance } in events.read() {
        use Player::*;
        let behavior = query.get(instance.entity()).unwrap();
        match *behavior {
            Idle => info!("Player is idle."),
            Walk => info!("Player has resumed walking ..."),
            _ => (),
        }
    }
}

fn player_stop(mut events: StopEvents<Player>) {
    for Stop { behavior, .. } in events.read() {
        use Player::*;
        match behavior {
            Walk => info!("Player has stopped walking."),
            Sing => info!("Player has stopped singing."),
            Rest => info!("Player has stopped resting."),
            _ => (),
        }
    }
}

fn handle_input(key: Res<ButtonInput<KeyCode>>, mut query: Query<BehaviorMut<Player>>) {
    use KeyCode::*;
    use Player::*;
    let mut player = query.single_mut();
    if key.just_pressed(KeyW) {
        player.start(Walk);
    } else if key.just_pressed(KeyR) {
        player.start(Rest);
    } else if key.just_pressed(KeyS) {
        player.start(Sing);
    } else if key.just_pressed(Backspace) {
        player.stop();
    } else if key.just_pressed(KeyX) {
        player.reset();
    }
}
