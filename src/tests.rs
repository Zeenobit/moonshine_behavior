use bevy::{ecs::system::RunSystemOnce, prelude::*};

use crate::prelude::*;

#[derive(Component, Default, Debug, PartialEq, Eq, Reflect)]
enum B {
    #[default]
    S0,
    S1,
    S2,
}

use B::*;

impl Behavior for B {
    fn allows_next(&self, next: &Self) -> bool {
        !matches!((self, next), (S0, S2))
    }
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, BehaviorPlugin::<B>::default()))
        .add_systems(Update, transition::<B>);
    app
}

#[test]
fn initial() {
    let mut app = app();
    let entity = app.world.spawn(BehaviorBundle::new(S0)).id();
    assert_eq!(*app.world.get::<B>(entity).unwrap(), S0);
}

#[test]
fn push() {
    let mut a = app();
    let e = a.world.spawn(BehaviorBundle::new(S0)).id();
    let r = a
        .world
        .run_system_once(|mut q: Query<BehaviorMut<B>>| q.single_mut().try_start(S1));
    a.update();
    assert!(r.poll().unwrap().is_ok());
    assert_eq!(*a.world.get::<B>(e).unwrap(), S1);
}

#[test]
fn push_fail() {
    let mut a = app();
    let e = a.world.spawn(BehaviorBundle::new(S0)).id();
    let r = a
        .world
        .run_system_once(|mut q: Query<BehaviorMut<B>>| q.single_mut().try_start(S2));
    a.update();
    assert!(r.poll().unwrap().is_err());
    assert_eq!(*a.world.get::<B>(e).unwrap(), S0);
}

#[test]
fn pop() {
    let mut a = app();
    let e = a.world.spawn(BehaviorBundle::new(S0)).id();
    let _ = a
        .world
        .run_system_once(|mut q: Query<BehaviorMut<B>>| q.single_mut().try_start(S1));
    a.update();
    a.world.run_system_once(|mut q: Query<BehaviorMut<B>>| {
        q.single_mut().stop();
    });
    a.update();
    assert_eq!(*a.world.get::<B>(e).unwrap(), S0);
}

#[test]
fn reset() {
    let mut a = app();
    let e = a.world.spawn(BehaviorBundle::new(S0)).id();
    let _ = a
        .world
        .run_system_once(|mut q: Query<BehaviorMut<B>>| q.single_mut().try_start(S1));
    a.update();
    let _ = a
        .world
        .run_system_once(|mut q: Query<BehaviorMut<B>>| q.single_mut().try_start(S2));
    a.update();
    a.world.run_system_once(|mut q: Query<BehaviorMut<B>>| {
        q.single_mut().reset();
    });
    a.update();
    assert_eq!(*a.world.get::<B>(e).unwrap(), S0);
}
