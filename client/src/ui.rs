use bevy::{input_focus::InputFocus, prelude::*};
use bevy_inspector_egui::egui::emath::GuiRounding;
use common::{Boat, MainCamera, Weapon, get_mut, primitives::{CustomTransform, DisplayScore, Level, Percent, PlayerStats, Size, UpgradeEvent, UpgradeRollbackEvent, WeaponCounter}, protocol::{Move, Rotate}, util::pixel};
use lightyear::prelude::{input::native::{ActionState, InputMarker}, *};

use crate::{BoatState, asset::{FontMap, SpriteMap, SpriteUiMap}, weapon::ChangeWeapon};

pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // app.insert_resource(BlockInput(false));  // true later once we have play button
        // app.add_plugins(DbgPlugin);
        app.add_systems(Startup, (spawn_progress_bar, spawn_upgrade_bar).after(crate::setup).chain());
        app.add_systems(Update, (recv_stats_update, receive_other_boat_update));

        app.add_observer(show_upgrade);
        app.add_observer(on_choose_upgrade);
        app.add_observer(on_upgrade_rollback);

        app.add_plugins(WeaponUiPlugin);
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
            font: fonts.get_long_lived("Aileron-Regular.otf").unwrap(),
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
                font: fonts.get_long_lived("Aileron-Regular.otf").unwrap(),
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
fn show_upgrade(  // this doesn't work if i add BlockInput etc
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
        debug_assert_eq!(*visibility, Visibility::Hidden, "Should only trigger if UpgradeBar is hidden");
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

struct WeaponUiPlugin;

impl Plugin for WeaponUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputFocus>();
        app.add_observer(spawn_weapon_selection_bar)
            .add_systems(Update, switch_weapon_selection_bar_brightness)
            .add_observer(change_weapon);
    }
}

#[derive(Debug, Component)]
struct WeaponSelection;

#[derive(Debug, Component)]
struct WeaponSelectionIndividualImage;

/// contains target
/// 
/// containint the weapon to avoid conflicts with actual weapon
#[derive(Debug, Component)]
struct WeaponSelectionIndividualBox(Weapon);

#[derive(Debug, Component)]
struct WeaponDataText;

// technically UI stuff
fn spawn_weapon_selection_bar(
    trigger: On<Add, WeaponCounter>,
    weapon_counter: Single<(Entity, &WeaponCounter), With<Controlled>>,
    mut commands: Commands,
    sprites: Res<SpriteUiMap>,
    fonts: Res<FontMap>
) {
    if weapon_counter.0 != trigger.entity {
        return;  // other's
    }

    let (_, weapon_counter) = weapon_counter.into_inner();

    let mut base_entity = commands.spawn((
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,  // not important
            margin: UiRect::top(auto()).with_left(auto()).with_right(auto()),
            // column_gap: px(20),
            padding: UiRect::all(px(5.5)),
            ..default()
        },
        WeaponSelection,
    ));
    let height = weapon_counter.weapons.keys().max_by(|this, other| {
        Ord::cmp(
            &sprites.get_size(**this).unwrap().height(),
            &sprites.get_size(**other).unwrap().height()
        )
    }).unwrap();
    let height = sprites.get_size(*height).unwrap().height() + PADDING_TOP as u32 * 2 + FONT_SIZE as u32 + ROW_GAP as u32;  // TODO .map().max()
    const FONT_SIZE: f32 = 15.0;
    const PADDING_TOP: i32 = 10;
    const ROW_GAP: i32 = 10;
    const PADDING: UiRect = UiRect::all(pixel(10)).with_left(pixel(5)).with_right(pixel(5));

    for (weapon, data) in weapon_counter.weapons.iter() {
        base_entity.with_children(|parent| {
            parent.spawn((
                // box
                Node {
                    height: px(height),
                    padding: PADDING,
                    flex_direction: FlexDirection::Column,  // image and text vertical
                    row_gap: px(ROW_GAP),  // so text doesn't get too close to image
                    ..default()
                },
                BackgroundColor(if weapon_counter.selected_weapon.unwrap() == *weapon {
                    PRESSED_BACKGROUND
                } else {
                    NORMAL_BACKGROUND
                }),
                Button,
                WeaponSelectionIndividualBox(*weapon)
            )).with_child((
                // image
                ImageNode {
                    image: sprites.image(),
                    texture_atlas: sprites.get(*weapon),
                    color: NORMAL,
                    ..default()
                },
                Node {
                    position_type: PositionType::Relative,
                    width: px(sprites.get_size(*weapon).unwrap().width()),
                    height: px(sprites.get_size(*weapon).unwrap().height()),
                    ..default()
                },
                WeaponSelectionIndividualImage
            ))
            .with_child((
                Text::new(format!("{}/{}", data.avaliable, data.max)),  // using indent
                TextFont {
                    font: fonts.get_long_lived("Aileron-Regular.otf").unwrap(),
                    font_size: FONT_SIZE,
                    ..default()
                },
                TextColor(if weapon_counter.selected_weapon.unwrap() == *weapon {
                    TEXT_SELECTED
                } else {
                    TEXT_LIGHT
                }),
                TextLayout {
                    justify: Justify::Right,
                    ..default()
                },
                WeaponDataText
            ));
        });
    }
}

const TEXT_LIGHT: Color = Color::linear_rgba(1.0, 1.0, 1.0, 0.5);
const TEXT_SELECTED: Color = Color::linear_rgba(1.0, 1.0, 1.0, 1.0);
// const NORMAL: Color = Color::linear_rgb(0.3, 0.3, 0.3);
const NORMAL: Color = Color::linear_rgb(0.5, 0.5, 0.5);
const NORMAL_BACKGROUND: Color = Color::linear_rgba(0.0, 0.0, 0.0, 0.0);

const HOVER: Color = Color::linear_rgb(0.5, 0.5, 0.5);
const HOVER_BACKGROUND: Color = Color::linear_rgba(0.3, 0.3, 0.3, 0.1);

const PRESSED: Color = Color::linear_rgb(1.0, 1.0, 1.0);
const PRESSED_BACKGROUND: Color = Color::linear_rgba(1.0, 1.0, 1.0, 0.1);
// TODO if exauhsted weapon, diff color

fn switch_weapon_selection_bar_brightness(
    mut input_focus: ResMut<InputFocus>,
    interaction_q: Query<
        (
            Entity,
            &Children,
            &Interaction,
            &mut BackgroundColor,
            &mut Button,
            &WeaponSelectionIndividualBox
        ),
        Changed<Interaction>
    >,
    mut images: Query<&mut ImageNode, With<WeaponSelectionIndividualImage>>,
    mut texts: Query<&mut TextColor, With<WeaponDataText>>,

    weapon_counter: Single<&WeaponCounter, With<Controlled>>,
    mut commands: Commands
) {
    for (entity, children, interaction, mut background, mut button, WeaponSelectionIndividualBox(weapon)) in interaction_q {
        let mut image = get_mut!(children, images).unwrap();
        let mut text_color = get_mut!(children, texts).unwrap();
        match *interaction {
            Interaction::Pressed => {
                if let Some(selected) = weapon_counter.selected_weapon
                    && selected == *weapon
                {
                    continue;  // no excessive event fires
                }
                input_focus.set(entity);
                commands.trigger(ChangeWeapon { target: *weapon });
                // colouring done in fn change_weapon to avoid ambiguity
                button.set_changed();
            }
            Interaction::Hovered => {
                if background.0 == PRESSED_BACKGROUND {
                    continue;  // already pressing
                }
                input_focus.set(entity);
                image.color = HOVER;
                background.0 = HOVER_BACKGROUND;
                button.set_changed();
            }
            Interaction::None => {
                if let Some(selected) = weapon_counter.selected_weapon
                    && selected == *weapon
                    && background.0 == PRESSED_BACKGROUND
                {
                    continue;  // if selected, don't release
                }

                input_focus.clear();

                image.color = NORMAL;  // more
                background.0 = NORMAL_BACKGROUND;
                text_color.0 = TEXT_LIGHT;
            }
        }
    }
}

fn change_weapon(
    trigger: On<ChangeWeapon>,
    mut backgrounds: Query<(&Children, &mut BackgroundColor, &WeaponSelectionIndividualBox)>,
    mut images: Query<&mut ImageNode, With<WeaponSelectionIndividualImage>>,
    mut texts: Query<&mut TextColor, With<WeaponDataText>>
) {
    {
        let (children, mut background, _box) =
            backgrounds.iter_mut()
                .find(|(_, background, _)| background.0 == PRESSED_BACKGROUND).unwrap();
        let mut image = get_mut!(children, images).unwrap();
        let mut text = get_mut!(children, texts).unwrap();

        background.0 = NORMAL_BACKGROUND;
        image.color = NORMAL;
        text.0 = TEXT_LIGHT;
    }

    let (children, mut background, _) = 
        backgrounds.iter_mut()
            .find(|(_, _, WeaponSelectionIndividualBox(weapon))| *weapon == trigger.target)
            .unwrap();
    let mut image = get_mut!(children, images).unwrap();
    let mut text_color = get_mut!(children, texts).unwrap();

    image.color = PRESSED;
    background.0 = PRESSED_BACKGROUND;
    text_color.0 = TEXT_SELECTED;
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