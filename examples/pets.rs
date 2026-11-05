use std::time::Duration;

use bevy::prelude::*;
use moonshine_behavior::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            BehaviorPlugin::<Bird>::default(),
            BehaviorPlugin::<Cat>::default(),
            BehaviorPlugin::<Dog>::default(),
        ))
        .add_plugins((
            sleep_plugin::<Bird>,
            sleep_plugin::<Cat>,
            sleep_plugin::<Dog>,
            eat_plugin::<Cat>,
            eat_plugin::<Dog>,
        ))
        .add_systems(Startup, spawn_pets)
        .add_systems(
            Update,
            (
                (bird, transition::<Bird>).chain(),
                (cat, transition::<Cat>).chain(),
                (dog, transition::<Dog>).chain(),
            ),
        )
        .run();
}

// Common states:

#[derive(Debug, Reflect)]
struct Sleep(Duration);

#[derive(Debug, Reflect)]
struct Eat(&'static str);

// Pets have some states in common, and some which are unique to them:

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
enum Bird {
    Idle,
    Sleep(Sleep),
    Chirp,
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
enum Cat {
    Idle,
    Sleep(Sleep),
    Eat(Eat),
    Purr,
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
enum Dog {
    Idle,
    Sleep { sleep: Sleep },
    Eat(Eat),
    Bark,
}

impl Behavior for Bird {}
impl Behavior for Cat {}
impl Behavior for Dog {}

impl_behavior_state!(Bird::Sleep as Sleep);
impl_behavior_state!(Cat::Sleep as Sleep);
impl_behavior_state!(Cat::Eat as Eat);
impl_behavior_state!(Dog::Sleep { sleep } as Sleep);
impl_behavior_state!(Dog::Eat as Eat);

#[derive(Component)]
struct Rested;

#[derive(Component)]
struct Satiated;

fn spawn_pets(mut commands: Commands) {
    commands.spawn(Bird::Idle);
    commands.spawn(Cat::Idle);
    commands.spawn(Dog::Idle);
}

// Each pet decides what to do next when idle, and handles its own unique state:

fn bird(query: Query<(Instance<Bird>, BehaviorMut<Bird>, Has<Rested>)>, mut commands: Commands) {
    for (instance, mut bird, rested) in query {
        match bird.current() {
            Bird::Idle if !rested => bird.start(Bird::Sleep(Sleep(Duration::from_secs(1)))),
            Bird::Idle => bird.start(Bird::Chirp),
            Bird::Chirp => {
                info!("{instance}: Chirp!");
                commands.entity(instance.entity()).remove::<Rested>();
                bird.stop();
            }
            _ => {}
        }
    }
}

fn cat(
    query: Query<(Instance<Cat>, BehaviorMut<Cat>, Has<Rested>, Has<Satiated>)>,
    mut commands: Commands,
) {
    for (instance, mut cat, rested, satiated) in query {
        match cat.current() {
            Cat::Idle if !satiated => cat.start(Cat::Eat(Eat("fish"))),
            Cat::Idle if !rested => cat.start(Cat::Sleep(Sleep(Duration::from_secs(3)))),
            Cat::Idle => cat.start(Cat::Purr),
            Cat::Purr => {
                info!("{instance}: Purr!");
                commands
                    .entity(instance.entity())
                    .remove::<(Rested, Satiated)>();
                cat.stop();
            }
            _ => {}
        }
    }
}

fn dog(
    query: Query<(Instance<Dog>, BehaviorMut<Dog>, Has<Rested>, Has<Satiated>)>,
    mut commands: Commands,
) {
    for (instance, mut dog, rested, satiated) in query {
        match dog.current() {
            Dog::Idle if !satiated => dog.start(Dog::Eat(Eat("kibble"))),
            Dog::Idle if !rested => dog.start(Dog::Sleep {
                sleep: Sleep(Duration::from_secs(2)),
            }),
            Dog::Idle => dog.start(Dog::Bark),
            Dog::Bark => {
                info!("{instance}: Woof!");
                commands
                    .entity(instance.entity())
                    .remove::<(Rested, Satiated)>();
                dog.stop();
            }
            _ => {}
        }
    }
}

// Common logic for any pet which can sleep:

fn sleep_plugin<T: BehaviorState<Sleep>>(app: &mut App) {
    app.add_observer(start_sleep::<T>)
        .add_observer(stop_sleep::<T>)
        .add_systems(Update, sleep::<T>);
}

#[derive(Component)]
struct SleepStart(Duration);

fn start_sleep<T: BehaviorState<Sleep>>(
    event: OnStart<T>,
    query: Query<BehaviorRef<T>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let entity = event.instance.entity();
    let Ok(behavior) = query.get(entity) else {
        return;
    };

    let Some(Sleep(duration)) = behavior[event.index].state() else {
        return;
    };

    info!("{}: Sleeping for {duration:?}", event.instance);
    commands.entity(entity).insert(SleepStart(time.elapsed()));
}

fn sleep<T: BehaviorState<Sleep>>(time: Res<Time>, query: Query<(BehaviorMut<T>, &SleepStart)>) {
    for (mut behavior, SleepStart(start)) in query {
        let Some(Sleep(duration)) = behavior.state() else {
            continue;
        };

        if time.elapsed() - *start >= *duration {
            behavior.stop();
        }
    }
}

fn stop_sleep<T: BehaviorState<Sleep>>(event: OnStop<T>, mut commands: Commands) {
    if !event.behavior.is_in_state() {
        return;
    }

    info!("{}: Woke up", event.instance);
    commands
        .entity(event.instance.entity())
        .insert(Rested)
        .remove::<SleepStart>();
}

// Common logic for any pet which can eat:

fn eat_plugin<T: BehaviorState<Eat>>(app: &mut App) {
    app.add_observer(start_eat::<T>)
        .add_observer(stop_eat::<T>)
        .add_systems(Update, eat::<T>);
}

fn start_eat<T: BehaviorState<Eat>>(event: OnStart<T>, query: Query<BehaviorRef<T>>) {
    let Ok(behavior) = query.get(event.instance.entity()) else {
        return;
    };

    let Some(Eat(food)) = behavior[event.index].state() else {
        return;
    };

    info!("{}: Eating {food}", event.instance);
}

fn eat<T: BehaviorState<Eat>>(query: Query<BehaviorMut<T>>) {
    for mut behavior in query {
        if behavior.is_in_state() {
            behavior.stop();
        }
    }
}

fn stop_eat<T: BehaviorState<Eat>>(event: OnStop<T>, mut commands: Commands) {
    if !event.behavior.is_in_state() {
        return;
    }

    commands.entity(event.instance.entity()).insert(Satiated);
}
