use bevy::prelude::*;
use bevy_inspector_egui::egui::emath::GuiRounding;
use common::{Boat, MainCamera, primitives::{CustomTransform, DisplayScore, Level, Percent, PlayerStats, Size, UpgradeEvent, UpgradeRollbackEvent}, protocol::{Move, Rotate}};
use lightyear::prelude::{input::native::{ActionState, InputMarker}, *};

use crate::{BoatState, asset::{FontMap, SpriteMap}};

pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // app.add_plugins(DbgPlugin);
        app.add_systems(Startup, (spawn_progress_bar, spawn_upgrade_bar).after(crate::setup).chain());
        app.add_systems(Update, (recv_stats_update, receive_other_boat_update));

        app.add_observer(show_upgrade);
        app.add_observer(on_choose_upgrade);
        app.add_observer(on_upgrade_rollback);
    }
}

/// shows the [`UpgradeBar`]
/// 
/// defaults currently to draw one level above current
/// 
/// diff with [`UpgradeEvent`]:
///     - triggered if [`DisplayScore::NewLevel`] is sent **and** [`UpgradeBar`] is hidden (which is hidden when user selects an upgrade)
#[derive(Debug, Event)]
struct ShowUpgrade;

fn recv_stats_update(
    mut rx: Single<&mut MessageReceiver<DisplayScore>>,
    current: Single<&PlayerStats>,
    upgrade_bar: Single<&Visibility, With<UpgradeBar>>,
    mut commands: Commands,
    mut progress_bar: Single<(&mut Text, &mut BackgroundGradient), With<ProgressBar>>,
) {
    let mut upgrade_bar_vis = *upgrade_bar;
    for msg in rx.receive() {
        // info!(?msg);
        match msg {
            DisplayScore::NewLevel(_level) => {
                // only show upgrade if upgrade bar is not visible right now
                if *upgrade_bar_vis == Visibility::Hidden {
                    commands.trigger(ShowUpgrade);
                    upgrade_bar_vis = &Visibility::Visible;
                }
            },
            DisplayScore::Percent(p) => {
                let (ref mut text, ref mut background) = *progress_bar;
                update_percent(p, current.level(), text, background);
            }
        }
    }
}

use common::util::InputExt as _;

/// text: X% to level next_level
/// 
/// in a linear-gradient background GUI
#[derive(Debug, Component)]
pub struct ProgressBar;

pub(super) fn spawn_progress_bar(
    mut commands: Commands,
    fonts: Res<FontMap>,
    current_level: Option<Single<&PlayerStats>>
) {
    let next_level = current_level.map(|c| c.level() + 1).unwrap_or(Level::Two);

    commands.spawn((
        Node {
            margin: UiRect {
                left: Val::Auto,
                right: Val::Auto,
                ..default()
            },

            height: Val::Auto,
            width: Val::Percent(30.),

            // smaller = less rounding
            border_radius: BorderRadius {
                bottom_left: Val::Px(8.0),
                bottom_right: Val::Px(8.0),
                ..default()
            },
            ..default()
        },
        TextLayout {
            justify: Justify::Center,  // crucial
            ..default()
        },
        TextFont {
            font: fonts.get_long_lived("regular.otf").unwrap(),
            font_size: 15.0,
            weight: FontWeight::BOLD,
            ..default()
        },
        Text(format!("0% to level {}", next_level)),

        Outline::new(Val::Px(1.0), Val::ZERO, Color::srgb_u8(102, 136, 102)),
        BackgroundGradient(vec![
            LinearGradient::to_right(vec![
                // blue
                ColorStop::percent(Color::srgb_u8(0, 132, 177), 0.),
                ColorStop::percent(Color::srgb_u8(0, 132, 177), 1.),
                // brown
                ColorStop::percent(Color::srgb_u8(62, 51, 51), 1.)
            ]).into()
        ]),

        ProgressBar,
    ));
}

/// updates `text` to `new_percent` to `current_level` + 1
/// 
/// ## Display
/// `"{new_percent} to level {current_level + 1}"`
/// and corresponding background
pub(super) fn update_percent(
    new_percent: Percent,
    current_level: Level,
    text: &mut Text,
    background: &mut BackgroundGradient
) {
    let next_level = current_level + 1;
    text.0 = format!("{new_percent}% to level {next_level}");

    // assumes only one LinearGradient in background
    if let Some(gradient) = background.0.get_mut(0)
        && let Gradient::Linear(linear_gradient) = gradient
        && linear_gradient.stops.len() == 3
    {
        linear_gradient.stops[1].point = Val::Percent(new_percent.to::<f32>());
        linear_gradient.stops[2].point = Val::Percent(new_percent.to::<f32>());
    } else {
        error_once!("Unexpected background configuration");
    }
}


/// text: Upgrade to level next_level
/// 
/// display next-level boats with observers on clicking on one of them
#[derive(Debug, Component)]
struct UpgradeBar;

/// spawns upgrade bar text and set visibility to hidden
fn spawn_upgrade_bar(
    fonts: Res<FontMap>,
    mut commands: Commands,
) {
    commands
        .spawn((
            Node {
                // pushes first boat down
                padding: UiRect::top(Val::Px(50.0)),
                margin: UiRect {
                    // center by X-axis
                    left: Val::Auto,
                    right: Val::Auto,
                    ..default()
                },
                // 20px gap between boats, assume all vertically placed
                row_gap: Val::Px(20.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            #[cfg(debug_assertions)]
            Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
            TextLayout {
                justify: Justify::Center,
                ..default()
            },
            TextFont {
                font: fonts.get_long_lived("regular.otf").unwrap(),
                font_size: 15.0,
                weight: FontWeight::BOLD,
                ..default()
            },
            Text("Upgrade to level N/A".to_owned()),
            Name::new("UpgradeBar"),
            UpgradeBar,
            Visibility::Hidden,
        ));
}
    
/// updates the upgrade selection bar [`UpgradeBar`]
/// 
/// - hide progress bar and set it to should-be-state after upgrading
/// - set pre-spawned upgrade bar to visible and update text
/// - replace upgrade bar's attached upgrade boats to appropriate ones
fn show_upgrade(
    _trigger: On<ShowUpgrade>,
    mut commands: Commands,
    player_stats: Single<&PlayerStats, With<Controlled>>,
    upgrade_bar: Single<(Entity, &mut Visibility, &mut Text), (With<UpgradeBar>, Without<ProgressBar>)>,
    sprites: Res<SpriteMap>,
    progress_bar: Single<(&mut Visibility, &mut Text, &mut BackgroundGradient), With<ProgressBar>>,
) {
    info!("Showed upgrade");
    let next_level = player_stats.level() + 1;

    {
        let (mut progress_vis, mut text, mut linear_gradient) = progress_bar.into_inner();
        *progress_vis = Visibility::Hidden;

        // pass in next level as current_level to set UI before user upgrading
        update_percent(0, next_level, &mut text, &mut linear_gradient);
    }

    let upgrade_bar = {
        let (entity, mut visibility, mut text) = upgrade_bar.into_inner();
        assert_eq!(*visibility, Visibility::Hidden, "Should only trigger if UpgradeBar is hidden");
        *visibility = Visibility::Visible;

        text.0 = format!("Upgrade to level {}", next_level);
        entity
    };

    commands.get_entity(upgrade_bar).unwrap()
        .despawn_children()  // clear previous upgrade boats TODO do it in select upgrade observer
        .with_children(|parent| {
            for boat in next_level.avaliable_boats() {
                parent.spawn((
                    Node {
                        // boats pile up
                        position_type: PositionType::Relative,

                        // render them 1/2 of actual size
                        width: Val::Px(boat.render_size().x / 2.0),
                        height: Val::Px(boat.render_size().y / 2.0),

                        // center of box
                        align_self: AlignSelf::Center,

                        ..default()
                    },
                    #[cfg(debug_assertions)]
                    Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
                    ImageNode {
                        image: sprites.image(),
                        texture_atlas: sprites.get(boat),
                        ..default()
                    },
                )).observe(move |
                    _trigger: On<Pointer<Click>>,
                    mut commands_o: Commands
                | {
                    info!("You clicked {:?}!", boat);
                    // currently not caring about click duration TODO
                    commands_o.trigger(UpgradeEvent {
                        target: boat
                    });
                });
            }
        });
}


// these two systems have friends in common::upgrade

// hide the upgradebar and disable click detections (possibly despawn?)
// un-hide progressbar
fn on_choose_upgrade(
    trigger: On<UpgradeEvent>,

    mut upgrade_bar: Single<&mut Visibility, With<UpgradeBar>>,
    mut progress_bar: Single<&mut Visibility, (With<ProgressBar>, Without<UpgradeBar>)>,

    mut sprite: Single<&mut Sprite, With<Controlled>>,
    sprites: Res<SpriteMap>
) {
    // sprite.image = sprites.image();
    sprite.custom_size = Some(trigger.target.render_size());
    let Some(ref mut texture_atlas) = sprite.texture_atlas else {
        panic!()
    };
    texture_atlas.index = sprites.get_index(trigger.target).unwrap_or_else(|| panic!("{:?} is not in the spritesheet", trigger.target));

    **upgrade_bar = Visibility::Hidden;
    **progress_bar = Visibility::Visible;
}

fn on_upgrade_rollback(
    trigger: On<UpgradeRollbackEvent>,

    mut sprite: Single<&mut Sprite, With<Controlled>>,
    sprites: Res<SpriteMap>
) {
    sprite.image = sprites.image();
    sprite.custom_size = Some(trigger.0.render_size());
    // note that we are not `= Some(sprites.get(trigger.target).unwrap())` everywhere, displaying a whole spritesheet is obvious enough
    sprite.texture_atlas = sprites.get(trigger.0);
}

/// update the sprite of non-controlled boats when they upgrade
fn receive_other_boat_update(
    query: Query<(&mut Sprite, &Boat), (Without<Controlled>, Changed<Boat>)>,
    sprites: Res<SpriteMap>
) {
    for (mut sprite, boat) in query {
        let Some(ref mut texture_atlas) = sprite.texture_atlas else { panic!() };
        texture_atlas.index = sprites.get_index(*boat).unwrap_or_else(|| panic!("{:?} is not in the spritesheet", boat));
        sprite.custom_size = Some(boat.render_size());
    }
}
#[allow(dead_code)]
struct DbgPlugin;

impl Plugin for DbgPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::spawn_dbg_gui.after(crate::setup))
            .add_systems(Update, Self::update_dbg_gui);
    }
}

#[allow(dead_code)]
impl DbgPlugin {

fn spawn_dbg_gui(mut commands: Commands, camera: Single<Entity, With<MainCamera>>) {
    commands.get_entity(camera.into_inner()).unwrap().insert(children![(
        Text2d::new("RotateInput: None\nSpeedInput: None\nState: Stopped\nPosition: None\nAltitude: None\nRotation: None\nSpeed: None\nScore: None"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        Transform::from_xyz(0.0, 280.0, common::OIL_RIG_Z + common::OCEAN_SURFACE.0 + 1.0),
    )]);
}
fn update_dbg_gui(
    mut text: Single<&mut Text2d>,
    rotate: Single<&ActionState<Rotate>, With<InputMarker<Rotate>>>,
    moves: Single<&ActionState<Move>, With<InputMarker<Move>>>,
    state: Res<State<BoatState>>,
    custom: Single<&CustomTransform, With<Controlled>>,
    transform: Single<&Transform, With<Controlled>>,
    player_score: Single<&PlayerStats, With<Controlled>>
) {
    let state = format!("{:?}", state.into_inner()).split("State(").last().unwrap().to_owned();

    let new_text = format!(
        "RotateInput: {}\nSpeedInput: {}\nState: {}\nPosition: {}\nAltitude: {}\nRotation: {}\nSpeed: {}\nScore: {}\nLevel: {:?}",
        rotate.0.0.map(|r| r.to_degrees().round()).unwrap_or(0.0),
        moves.0.0.map(|r| r.get_knots().round()).unwrap_or(0.0),
        state.chars().take(state.len() - 1).collect::<String>(),
        custom.position.0.round(),
        transform.translation.z.round_to_pixels(10.0),
        custom.rotation.to_degrees().round(),
        custom.speed.get_knots().round(),
        player_score.score(),
        player_score.level()
    );

    if new_text != text.0 {
        text.0 = new_text;
    }
}

}