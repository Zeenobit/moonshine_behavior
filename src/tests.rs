#![allow(clippy::type_complexity)]

use bevy::{ecs::system::RunSystemOnce, prelude::*};

use crate::events::Error;
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

    fn on_start(&self, _: Option<&Self>, mut commands: InstanceCommands<Self>) {
        match self {
            A => commands.insert(TA),
            B => commands.insert(TB),
            C => commands.insert(TC),
            D => commands.insert(TD),
        };
    }

    fn on_stop(&self, _: &Self, mut commands: InstanceCommands<Self>) {
        match self {
            A => commands.remove::<TA>(),
            B => commands.remove::<TB>(),
            C => commands.remove::<TC>(),
            D => commands.remove::<TD>(),
        };
    }
}

#[derive(Component)]
struct TA;

#[derive(Component)]
struct TB;

#[derive(Component)]
struct TC;

#[derive(Component)]
struct TD;

#[derive(Resource)]
struct ErrorTriggered;

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
            .run_system_once(|q: Single<BehaviorRef<T>, With<TA>>| { **q })
            .unwrap(),
        A
    );
}

#[test]
fn initial_load() {
    let mut app = app();
    app.world_mut().spawn((
        C,
        // Simulate loading from saved data:
        crate::Memory {
            stack: [A, B].into(),
        },
    ));
    app.update();

    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TB>, With<TC>)>| { **q })
            .unwrap(),
        C
    );
}

#[test]
fn push() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TB>)>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), B)
    );
}

#[test]
fn push_error() {
    let mut app = app();
    let e = app.world_mut().spawn((A, Next(C))).id();

    app.add_observer(
        move |event: On<Error<T>>,
              q: Single<BehaviorRef<T>, (With<TA>, Without<TC>)>,
              mut commands: Commands| {
            assert_eq!(**q, A);
            assert_eq!(event.instance, e);
            commands.insert_resource(ErrorTriggered);
        },
    );

    app.update();
    assert!(app.world().contains_resource::<ErrorTriggered>());
}

#[test]
fn pop() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();
    app.world_mut()
        .run_system_once(|mut q: Single<BehaviorMut<T>, (With<TA>, With<TB>)>| {
            q.stop();
        })
        .unwrap();
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, Without<TB>)>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (None, A)
    );
}

#[test]
fn pop_initial() {
    let mut app = app();
    let e = app.world_mut().spawn((A, Previous::<T>)).id();

    app.add_observer(
        move |event: On<Error<T>>, q: Single<BehaviorRef<T>, With<TA>>, mut commands: Commands| {
            assert_eq!(**q, A);
            assert_eq!(event.instance, e);
            commands.insert_resource(ErrorTriggered);
        },
    );

    app.update();
    assert!(app.world().contains_resource::<ErrorTriggered>());
}

#[test]
fn queue() {
    let mut app = app();
    app.world_mut().spawn((
        A,
        TransitionQueue::start(B) // A -> B
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
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TB>)>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), B)
    );

    app.update(); // B -> C
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, With<TC>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(B), C)
    );

    app.update(); // C -> D
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, With<TC>, With<TD>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
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
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, With<TC>, Without<TD>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(B), C)
    );

    app.update(); // C -> B
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, Without<TC>, Without<TD>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(A), B)
    );

    app.update(); // B -> A
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, Without<TB>, Without<TC>, Without<TD>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (None, A)
    );

    app.update(); // A -> D
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, Without<TB>, Without<TC>, With<TD>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(A), D)
    );

    // Queue should be done, check again to be sure:
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, Without<TB>, Without<TC>, With<TD>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(A), D)
    );
    assert!(app
        .world_mut()
        .run_system_once(|q: Query<&TransitionQueue<T>>| { q.single().is_err() })
        .unwrap());
}

#[test]
fn chain() {
    let mut app = app();

    // A -> B -> C -> D
    app.world_mut()
        .spawn((A, TransitionQueue::chain([B, C, D])));

    app.update(); // A
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, With<TA>>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (None, A)
    );

    app.update(); // A -> B
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TB>)>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), B)
    );

    app.update(); // A -> B -> C
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, With<TC>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(B), C)
    );

    app.update(); // A -> B -> C -> D
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, With<TC>, With<TD>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(C), D)
    );

    // Queue should be done, check again to be sure:
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, With<TC>, With<TD>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(C), D)
    );
    assert!(app
        .world_mut()
        .run_system_once(|q: Query<&TransitionQueue<T>>| { q.single().is_err() })
        .unwrap());
}

#[test]
fn sequence() {
    let mut app = app();

    // A -> B, A -> D
    app.world_mut()
        .spawn((A, TransitionQueue::sequence([B, D])));

    app.update(); // A
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorMut<T>, With<TA>>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (None, A)
    );

    app.update(); // A -> B
    assert_eq!(
        app.world_mut()
            .run_system_once(|mut q: Single<BehaviorMut<T>, (With<TA>, With<TB>)>| {
                q.stop();
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), B)
    );

    app.update(); // A
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, With<TA>>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (None, A)
    );

    app.update(); // A -> D
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TD>)>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), D)
    );

    // Queue should be done, check again to be sure:
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TD>)>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), D)
    );
    assert!(app
        .world_mut()
        .run_system_once(|q: Query<&TransitionQueue<T>>| { q.single().is_err() })
        .unwrap());
}

#[test]
fn interrupt_start() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TB>)>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), B)
    );

    app.world_mut()
        .run_system_once(|mut q: Single<BehaviorMut<T>>| {
            q.interrupt_start(D);
        })
        .unwrap();
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, Without<TB>, With<TD>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(A), D)
    );
}

#[test]
fn interrupt_resume() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();

    app.world_mut()
        .run_system_once(|mut q: Single<BehaviorMut<T>>| {
            q.start(C);
        })
        .unwrap();
    app.update();

    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, With<TC>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(B), C)
    );

    app.world_mut()
        .run_system_once(|mut q: Single<BehaviorMut<T>>| {
            let index = q.current_index().previous().unwrap();
            q.interrupt_resume(index);
        })
        .unwrap();
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, Without<TC>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(A), B)
    );
}

#[test]
fn interrupt_stop() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();

    app.world_mut()
        .run_system_once(|mut q: Single<BehaviorMut<T>>| {
            q.start(C);
        })
        .unwrap();
    app.update();

    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, With<TC>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(B), C)
    );

    app.world_mut()
        .run_system_once(|mut q: Single<BehaviorMut<T>>| {
            let index = q.current_index().previous().unwrap();
            q.interrupt_stop(index);
        })
        .unwrap();
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, Without<TB>, Without<TC>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (None, A)
    );
}

#[test]
fn interrupt_start_push() {
    let mut app = app();
    app.world_mut().spawn((A, Next(B)));
    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TB>)>| {
                (q.previous().copied(), *q.current())
            })
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
            .run_system_once(
                |q: Single<BehaviorRef<T>, (With<TA>, With<TB>, With<TC>)>| {
                    (q.previous().copied(), *q.current())
                }
            )
            .unwrap(),
        (Some(B), C)
    );
}

#[test]
fn interrupt_error() {
    let mut app = app();
    let e = app.world_mut().spawn((A, Next(B))).id();

    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TB>)>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), B)
    );

    app.world_mut()
        .run_system_once(|mut q: Single<BehaviorMut<T>>| {
            q.interrupt_start(A);
        })
        .unwrap();

    app.add_observer(
        move |event: On<Error<T>>, q: Single<BehaviorRef<T>, With<TA>>, mut commands: Commands| {
            assert_eq!(**q, B);
            assert_eq!(event.instance, e);
            commands.insert_resource(ErrorTriggered);
        },
    );

    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TB>)>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), B)
    );
}

#[test]
fn try_start() {
    let mut app = app();
    app.world_mut().spawn(A);
    app.update();

    assert_eq!(
        app.world_mut()
            .run_system_once(|mut q: Single<BehaviorMut<T>>| {
                q.try_start(B).unwrap();
                q.try_start(D)
            })
            .unwrap(),
        Err(D)
    );

    app.update();
    assert_eq!(
        app.world_mut()
            .run_system_once(|q: Single<BehaviorRef<T>, (With<TA>, With<TB>)>| {
                (q.previous().copied(), *q.current())
            })
            .unwrap(),
        (Some(A), B)
    );
}
