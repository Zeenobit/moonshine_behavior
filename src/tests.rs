use bevy::prelude::*;

use crate::{transition, Behavior, BehaviorBundle, BehaviorPlugin, Next, Previous, Reset};

#[derive(Component, Default, Debug, PartialEq, Eq, Reflect, FromReflect)]
enum B {
    #[default]
    S0,
    S1,
    S2,
}

use B::*;

impl Behavior for B {}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugin(BehaviorPlugin::<B>::default())
        .add_system(transition::<B>);
    app
}

pub trait UpdateWith {
    fn update_with<R, F>(self, f: F) -> R
    where
        F: Fn(&mut World) -> R;
}

impl UpdateWith for &mut App {
    fn update_with<R, F>(self, f: F) -> R
    where
        F: Fn(&mut World) -> R,
    {
        let result = f(&mut self.world);
        self.update();
        result
    }
}

#[test]
fn initial() {
    let mut app = app();

    let entity = app.world.spawn(BehaviorBundle::new(S0)).id();

    assert_eq!(*app.world.get::<B>(entity).unwrap(), S0);
}

#[test]
fn push() {
    let mut app = app();
    let entity = app.world.spawn(BehaviorBundle::new(S0)).id();

    app.update_with(|world| {
        world.entity_mut(entity).insert(Next(S1));
    });

    assert_eq!(*app.world.get::<B>(entity).unwrap(), S1);
}

#[test]
fn pop() {
    let mut app = app();
    let entity = app.world.spawn(BehaviorBundle::new(S0)).id();

    app.update_with(|world| {
        world.entity_mut(entity).insert(Next(S1));
    });
    app.update_with(|world| {
        world.entity_mut(entity).insert(Previous::<B>);
    });

    assert_eq!(*app.world.get::<B>(entity).unwrap(), S0);
}

#[test]
fn reset() {
    let mut app = app();
    let entity = app.world.spawn(BehaviorBundle::new(S0)).id();

    app.update_with(|world| {
        world.entity_mut(entity).insert(Next(S1));
    });
    app.update_with(|world| {
        world.entity_mut(entity).insert(Next(S2));
    });
    app.update_with(|world| {
        world.entity_mut(entity).insert(Reset::<B>);
    });

    assert_eq!(*app.world.get::<B>(entity).unwrap(), S0);
}
