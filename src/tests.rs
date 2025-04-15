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
        match_next! {
            self => next,
            A => B | D,
            B => C,
            C => D,
        }
    }

    fn filter_yield(&self, next: &Self) -> bool {
        match_next! {
            self => next,
            B => D,
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
            .run_system_once(|q: Single<BehaviorRef<T>>| { **q })
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
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(A), B)
    );
}

#[test]
fn push_error() {
    let mut app = app();
    app.world_mut().spawn((A, Next(C)));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|mut e: TransitionEvents<T>, q: Single<BehaviorRef<T>>| {
                assert!(matches!(
                    e.read().next(),
                    Some(TransitionEvent::Error { .. })
                ));
                (q.previous().copied(), *q.current())
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
        .run_system_once(|mut q: Single<BehaviorMut<T>>| {
            q.stop();
        })
        .unwrap();
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
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
            .run_system_once(|q: Single<BehaviorRef<T>>| { **q })
            .unwrap(),
        A
    );
}

#[test]
fn sequence() {
    let mut app = app();
    app.world_mut().spawn((
        A,
        TransitionSequence::start(B) // A -> B
            .then(C) // B -> C
            .then_wait_for(D) // C -> D -> C
            .then_stop() //  C -> B
            .then_stop() // B -> A
            .then(D), // A -> D
    ));
    app.update(); // None -> A
    app.update(); // A -> B
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(A), B)
    );

    app.update(); // B -> C
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(B), C)
    );

    app.update(); // C -> D
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(C), D)
    );

    // Stop D
    app.world_mut()
        .run_system_once(|mut q: Single<BehaviorMut<T>>| {
            q.stop();
        })
        .unwrap();
    app.update(); // D -> C
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(B), C)
    );

    app.update(); // C -> B
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(A), B)
    );

    app.update(); // B -> A
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (None, A)
    );

    app.update(); // A -> D
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(A), D)
    );

    // Sequence should be done, check again to be sure:
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(A), D)
    );
    assert!(app
        .world_mut()
        .run_system_once(|q: Query<&TransitionSequence<T>>| { q.get_single().is_err() })
        .unwrap());
}

#[test]
fn interrupt() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(A), B)
    );

    app.world_mut()
        .run_system_once(|mut q: Query<BehaviorMut<T>>| {
            q.single_mut().interrupt_start(D);
        })
        .unwrap();
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(A), D)
    );
}

#[test]
fn interrupt_push() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(A), B)
    );

    app.world_mut()
        .run_system_once(|mut q: Single<BehaviorMut<T>>| {
            q.interrupt_start(C);
        })
        .unwrap();
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(B), C)
    );
}

#[test]
fn interrupt_error() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>>| { (q.previous().copied(), *q.current()) })
            .unwrap(),
        (Some(A), B)
    );

    app.world_mut()
        .run_system_once(|mut q: Single<BehaviorMut<T>>| {
            q.interrupt_start(A);
        })
        .unwrap();
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|mut e: TransitionEvents<T>, q: Single<BehaviorRef<T>>| {
                assert!(matches!(
                    e.read().skip(2).next(),
                    Some(TransitionEvent::Error { .. })
                ));
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), B)
    );
}
