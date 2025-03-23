use bevy::{ecs::system::RunSystemOnce, prelude::*};

use crate::prelude::*;

use self::T::*;

#[derive(Component, Default, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
enum T {
    #[default]
    A,
    B,
    C,
    D,
}

impl Behavior for T {
    fn filter_next(&self, next: &Self) -> bool {
        match (self, next) {
            (A, B) | (B, C) | (A, D) => true,
            _ => false,
        }
    }
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, BehaviorPlugin::<T>::default()))
        .add_systems(Update, transition::<T>);
    app
}

#[test]
fn initial() {
    let mut app = app();
    app.world_mut().spawn(A);
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Query<BehaviorRef<T>>| { *q.single() })
            .unwrap(),
        A
    );
}

#[test]
fn push() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Query<BehaviorRef<T>>| {
                let behavior = q.single();
                (behavior.previous().copied(), *behavior.current())
            })
            .unwrap(),
        (Some(A), B)
    );
}

#[test]
fn push_reject() {
    let mut app = app();
    app.world_mut().spawn((A, Next(C)));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Query<BehaviorRef<T>>| {
                let behavior = q.single();
                (behavior.previous().copied(), *behavior.current())
            })
            .unwrap(),
        (None, A)
    );
}

#[test]
fn pop() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();

    app.world_mut()
        .run_system_once(|mut q: Query<BehaviorMut<T>>| {
            q.single_mut().stop();
        })
        .unwrap();
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Query<BehaviorRef<T>>| {
                let behavior = q.single();
                (behavior.previous().copied(), *behavior.current())
            })
            .unwrap(),
        (None, A)
    );
}

#[test]
fn pop_initial() {
    let mut app = app();
    app.world_mut().spawn((A, Previous::<T>));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Query<BehaviorRef<T>>| { *q.single() })
            .unwrap(),
        A
    );
}

#[test]
fn sequence() {
    let mut app = app();
    app.world_mut().spawn((A, Sequence::new([B, D])));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Query<BehaviorRef<T>>| {
                let behavior = q.single();
                (behavior.previous().copied(), *behavior.current())
            })
            .unwrap(),
        (Some(A), B)
    );

    app.world_mut()
        .run_system_once(|mut q: Query<BehaviorMut<T>>| {
            q.single_mut().stop();
        })
        .unwrap();
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Query<BehaviorRef<T>>| {
                let behavior = q.single();
                (behavior.previous().copied(), *behavior.current())
            })
            .unwrap(),
        (Some(A), D)
    );

    // Sequence should be done, check again to be sure:
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Query<BehaviorRef<T>>| {
                let behavior = q.single();
                (behavior.previous().copied(), *behavior.current())
            })
            .unwrap(),
        (Some(A), D)
    );
}
