use bevy::{ecs::system::RunSystemOnce, prelude::*};

use moonshine_behavior::prelude::*;

#[derive(Component, Default, Debug, PartialEq, Eq, Reflect)]
enum B {
    #[default]
    S0,
    S1,
    S2,
    S3,
}

use B::*;

impl Behavior for B {
    fn allows_next(&self, next: &Self) -> bool {
        matches!((self, next), (S0, S1) | (S1, S2) | (S2, S3))
    }

    fn stopped(&self) -> Option<Self> {
        match self {
            S2 => Some(S3),
            _ => None,
        }
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
    let mut a = app();
    let e = a.world_mut().spawn((S0, Controller::<B>::default())).id();
    assert_eq!(*a.world().get::<B>(e).unwrap(), S0);
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_started() })
        .unwrap());

    a.update();
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_stable() })
        .unwrap());
}

#[test]
fn push() {
    let mut a = app();
    let e = a.world_mut().spawn((S0, Controller::<B>::default())).id();
    let r = a
        .world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| q.single_mut().try_start(S1))
        .unwrap();
    a.update();
    assert!(r.poll().unwrap().is_ok());
    assert_eq!(*a.world().get::<B>(e).unwrap(), S1);
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_started() })
        .unwrap());

    a.update();
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_stable() })
        .unwrap());
}

#[test]
fn push_fail() {
    let mut a = app();
    let e = a.world_mut().spawn((S0, Controller::<B>::default())).id();
    let r = a
        .world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| q.single_mut().try_start(S2))
        .unwrap();
    a.update();
    assert!(r.poll().unwrap().is_err());
    assert_eq!(*a.world_mut().get::<B>(e).unwrap(), S0);
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_stable() })
        .unwrap());
}

#[test]
fn pop() {
    let mut a = app();
    let e = a.world_mut().spawn((S0, Controller::<B>::default())).id();
    let _ = a
        .world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| q.single_mut().try_start(S1));
    a.update();
    a.world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| {
            q.single_mut().stop();
        })
        .unwrap();
    a.update();
    assert_eq!(*a.world().get::<B>(e).unwrap(), S0);
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&mut Controller<B>>| { q.single().is_resumed() })
        .unwrap());

    a.update();
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_stable() })
        .unwrap());
}

#[test]
fn reset() {
    let mut a = app();
    let e = a.world_mut().spawn((S0, Controller::<B>::default())).id();
    let _ = a
        .world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| q.single_mut().try_start(S1));
    a.update();
    let _ = a
        .world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| q.single_mut().try_start(S2));
    a.update();
    a.world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| {
            q.single_mut().reset();
        })
        .unwrap();
    a.update();
    assert_eq!(*a.world().get::<B>(e).unwrap(), S0);
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_resumed() })
        .unwrap());

    a.update();
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_stable() })
        .unwrap());
}

#[test]
fn chain() {
    let mut a = app();
    let e = a.world_mut().spawn((S0, Controller::<B>::default())).id();
    let _ = a
        .world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| q.single_mut().try_start(S1));
    a.update();

    let _ = a
        .world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| q.single_mut().try_start(S2));
    a.update();
    assert_eq!(*a.world().get::<B>(e).unwrap(), S2);

    a.world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| {
            q.single_mut().stop();
        })
        .unwrap();
    a.update();
    assert_eq!(*a.world().get::<B>(e).unwrap(), S3);
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_started() })
        .unwrap());

    a.update();
    assert_eq!(*a.world().get::<B>(e).unwrap(), S3);
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_stable() })
        .unwrap());

    a.world_mut()
        .run_system_once(|mut q: Query<&mut Controller<B>>| {
            q.single_mut().stop();
        })
        .unwrap();
    a.update();
    assert_eq!(*a.world().get::<B>(e).unwrap(), S2);
    assert!(a
        .world_mut()
        .run_system_once(|q: Query<&Controller<B>>| { q.single().is_resumed() })
        .unwrap());
}
