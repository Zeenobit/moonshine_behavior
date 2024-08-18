use bevy::prelude::*;
use moonshine_behavior::prelude::*;

const HELP_TEXT: &str = "This example simulates the state of a bird.
Each button corresponds to an action available to the bird. A red button means the action is not allowed.
When idle, the bird can sleep, chirp, or fly. When sleeping, the bird cannot chirp, or fly.
The bird may only chirp if flying or idle.";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bird Behavior".to_string(),
                resolution: (800., 300.).into(),
                ..default()
            }),
            ..default()
        }))
        // Add the BehaviorPlugin for Bird behavior.
        // This plugin is required for the behavior system to work with a behavior type.
        .add_plugins(behavior_plugin::<Bird>())
        // Add the transition system for Bird behavior
        // Behavior changes happen in this system. Register your systems before or after it as needed.
        .add_systems(Update, transition::<Bird>)
        // ... other systems ...
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (update_text, update_buttons).after(transition::<Bird>),
        )
        .add_systems(Update, on_button_clicked.before(transition::<Bird>))
        .run();
}

// Define Bird behavior as an enum with all of its possible states.
#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
enum Bird {
    #[default]
    Idle,
    Fly,
    Sleep,
    Chirp,
}

// Implement Behavior for Bird to describe behavior transitions as required:
// 1. When idle, the bird can sleep, chirp, or fly.
// 2. When sleeping, the bird cannot chirp, or fly.
// 3. The bird may only chirp if flying or idle.
impl Behavior for Bird {
    fn allows_next(&self, next: &Self) -> bool {
        use Bird::*;
        match self {
            Idle => matches!(next, Sleep | Fly | Chirp),
            Fly => matches!(next, Chirp),
            Sleep | Chirp => false,
        }
    }
}

// A marker component for the message text.
#[derive(Component)]
struct Message;

// A marker component for the buttons.
#[derive(Component)]
enum Action {
    Fly,
    Sleep,
    Chirp,
    Stop,
    Reset,
}

// Spawn a Bird and setup UI.
fn setup(mut commands: Commands) {
    commands.spawn(BehaviorBundle::<Bird>::default());
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Start,
                padding: UiRect::all(Val::Px(20.)),
                ..Default::default()
            },
            background_color: BackgroundColor(bevy::color::palettes::css::GRAY.into()),
            ..Default::default()
        })
        .with_children(|root| {
            root.spawn(TextBundle {
                text: Text::from_section(
                    HELP_TEXT,
                    TextStyle {
                        font_size: 20.,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                style: Style {
                    margin: UiRect::bottom(Val::Px(20.)),
                    ..default()
                },
                ..default()
            });
            root.spawn((
                Message,
                TextBundle {
                    text: Text::from_section(
                        "",
                        TextStyle {
                            font_size: 25.,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    ..default()
                },
            ));
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Start,
                    ..default()
                },
                ..default()
            })
            .with_children(|parent| {
                spawn_button(parent, "Fly", Action::Fly);
                spawn_button(parent, "Sleep", Action::Sleep);
                spawn_button(parent, "Chirp", Action::Chirp);
                spawn_button(parent, "Stop", Action::Stop);
                spawn_button(parent, "Reset", Action::Reset);
            });
        });
}

// Update the message text based on the current state of the Bird.
fn update_text(
    query: Query<(&Bird, Previous<Bird>), Changed<Bird>>,
    mut message: Query<&mut Text, With<Message>>,
) {
    use Bird::*;
    if let Ok((bird, previous)) = query.get_single() {
        let mut text = message.single_mut();
        text.sections[0].value = match bird {
            Idle => "Bird is idle.",
            Fly => "Bird is flying.",
            Sleep => "Bird is sleeping.",
            Chirp => {
                if let Some(Fly) = previous.get() {
                    "Bird is chirping while flying."
                } else {
                    "Bird is chirping."
                }
            }
        }
        .to_string();
    }
}

// Update the button colors based on the current state of the Bird.
fn update_buttons(
    query: Query<&Bird, Changed<Bird>>,
    mut buttons: Query<(&Action, &mut BackgroundColor), With<Button>>,
) {
    use Bird::*;
    if let Ok(behavior) = query.get_single() {
        for (action, mut color) in buttons.iter_mut() {
            let is_allowed = match action {
                Action::Fly => behavior.allows_next(&Fly),
                Action::Sleep => behavior.allows_next(&Sleep),
                Action::Chirp => behavior.allows_next(&Chirp),
                _ => true,
            };
            color.0 = if is_allowed {
                bevy::color::palettes::css::DARK_GREEN.into()
            } else {
                bevy::color::palettes::css::RED.into()
            };
        }
    }
}

// Modify the Bird behavior based on button clicks.
fn on_button_clicked(
    query: Query<(&Action, &Interaction), Changed<Interaction>>,
    mut bird: Query<&mut Transition<Bird>>,
) {
    use Bird::*;
    let mut transition = bird.single_mut();
    for (action, interaction) in query.iter() {
        if let Interaction::Pressed = interaction {
            match action {
                Action::Fly => transition.try_start(Fly).forget(),
                Action::Sleep => transition.try_start(Sleep).forget(),
                Action::Chirp => transition.try_start(Chirp).forget(),
                Action::Stop => transition.stop(),
                Action::Reset => transition.reset(),
            }
        }
    }
}

// Spawn a button with the given text and action.
fn spawn_button(parent: &mut ChildBuilder, text: impl Into<String>, action: Action) {
    parent
        .spawn((
            action,
            ButtonBundle {
                background_color: bevy::color::palettes::css::DARK_GRAY.into(),
                style: Style {
                    margin: UiRect::all(Val::Px(5.)),
                    padding: UiRect::new(Val::Px(10.), Val::Px(10.), Val::Px(5.), Val::Px(5.)),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|fly_button| {
            fly_button.spawn(TextBundle {
                text: Text::from_section(
                    text,
                    TextStyle {
                        font_size: 20.,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                ..default()
            });
        });
}
