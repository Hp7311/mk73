use bevy::{ecs::{query::QueryData}, input_focus::InputFocus, prelude::*};
use bevy_inspector_egui::egui::emath::GuiRounding;
use common::{Boat, UpgradeEventCommonFinished, Weapon, get_mut, primitives::{CustomTransform, DisplayScore, Level, Percent, PlayerStats, Size, UpgradeEvent, UpgradeRollbackEvent, WeaponCounter, WeaponData}, protocol::{Move, Rotate}, util::{BlockInput, pixel, zip_longest}};
use lightyear::prelude::{input::native::{ActionState, InputMarker}, *};

use crate::{BoatState, asset::{SpriteMap, SpriteUiMap}, weapon::ChangeWeapon};

pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // app.insert_resource(BlockInput(false));  // true later once we have play button
        app.add_plugins(DbgPlugin);
        app.add_systems(Startup, (spawn_progress_bar, spawn_upgrade_bar).after(crate::setup).chain());
        app.add_systems(Update, (recv_stats_update, receive_other_boat_update));

        app.add_observer(show_upgrade);
        app.add_observer(on_choose_upgrade);
        app.add_observer(on_upgrade_rollback);

        app.add_plugins(WeaponUiPlugin);

        app.insert_state(AfterUpgradeDontClearMoveState::NoNeed);

        app.insert_resource(BlockInput(false));
        
        // pre update because:
        // uh
        app.add_systems(FixedPreUpdate, update_block_input_to_true.run_if(resource_equals(BlockInput(false))))
            // post update because:
            // if a interaction finished before the current frame, systems may catch the event and see BlockInput(false)
            .add_systems(FixedPostUpdate, update_block_input_to_false.run_if(resource_equals(BlockInput(true))));
    }
}

fn update_block_input_to_true(interactions: Query<&Interaction, Changed<Interaction>>, mut block_input: ResMut<BlockInput>) {
    // should BlockInput target Hovered?
    if interactions.into_iter().find(|i| matches!(i, Interaction::Pressed)).is_some() {
        trace!("Interacting with UI, blocking gameplay actions");
        block_input.0 = true;
    }
}
fn update_block_input_to_false(interactions: Query<&Interaction, Changed<Interaction>>, mut block_input: ResMut<BlockInput>) {
    // if any changes to hovered/none
    // still checking because Changed records all mutable derefs
    if interactions.into_iter().find(|i| matches!(i, Interaction::Hovered | Interaction::None)).is_some() {
        trace!("Finished interacting with the UI, re-enabling gameplay actions");
        block_input.0 = false;
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
    current: Single<&PlayerStats, With<Controlled>>,
    upgrade_bar: Single<&Visibility, With<UpgradeBar>>,
    mut commands: Commands,
    mut progress_bar: Single<(&mut Text, &mut BackgroundGradient), With<ProgressBar>>,
) {
    let mut upgrade_bar_vis = **upgrade_bar;
    for msg in rx.receive() {
        debug!(stats_update = ?msg);
        match msg {
            DisplayScore::NewLevel(_target) => {
                #[cfg(debug_assertions)]
                if current.level() == _target {
                    // this does happen sometimes
                    warn!("Server behind on updates (not expected since we used UpgradeSet to make upgrade components run before )");
                    continue;
                }
                // only show upgrade if upgrade bar is not visible right now
                if upgrade_bar_vis == Visibility::Hidden {
                    commands.trigger(ShowUpgrade);
                    debug!("Triggered ShowUpgrade because UpgradeBar is hidden");
                    upgrade_bar_vis = Visibility::Visible;
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
            font: FontSource::SansSerif,
            font_size: FONT_SIZE,
            weight: FontWeight::NORMAL,
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
#[derive(Debug, Component)]
struct UpgradeText;

/// spawns upgrade bar text and set visibility to hidden
fn spawn_upgrade_bar(mut commands: Commands) {
    commands
        .spawn((
            Node {
                // pushes first boat down
                // padding: UiRect::top(Val::Px(50.0)),
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
            children![(
                Node {
                    padding: UiRect::top(px(10)).with_bottom(px(10)),
                    ..default()
                },
                TextLayout {
                    justify: Justify::Center,
                    ..default()
                },
                TextFont {
                    font: FontSource::SansSerif,
                    font_size: FONT_SIZE,
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                Text("Upgrade to level N/A".to_owned()),
                UpgradeText
            )],
            Name::new("UpgradeBar"),
            UpgradeBar,
            Visibility::Hidden,
        ));
}

/// signature:
/// - `(parent: &mut /* parent spawner */, boat: Boat, ?img = image: NodeImage) -> EntityCommands`
/// - `(parent: &mut /* parent spawner */, boat: Boat, ?sprites = sprites: SpriteMap) -> EntityCommands`
macro_rules! spawn_upgrade_ui_boat {
    ($parent:expr, $boat:expr, ?img = $image:expr) => {
        $parent.spawn((
            Node {
                // boats pile up
                position_type: PositionType::Relative,

                // render them 1/2 of actual size
                width: Val::Px($boat.render_size().x / 2.0),
                height: Val::Px($boat.render_size().y / 2.0),

                // center of box
                align_self: AlignSelf::Center,

                ..default()
            },
            #[cfg(debug_assertions)]
            Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK),
            Button,  // to block user input
            UpgradeOption($boat),
            $image
        )).observe(click_upgrade_observer)
    };
    ($parent:expr, $boat:expr, ?sprites = $sprites:expr) => {
        spawn_upgrade_ui_boat!($parent, $boat, ?img = bevy::prelude::ImageNode {
            image: $sprites.image(),
            texture_atlas: $sprites.get($boat),
            ..Default::default()
        })
    }
}

#[derive(Debug, Component)]
struct UpgradeOption(Boat);

fn click_upgrade_observer(
    trigger: On<Pointer<Click>>,
    upgrade_options: Query<&UpgradeOption>,
    mut commands: Commands
) {
    let boat = upgrade_options.get(trigger.entity).unwrap().0;
    debug!("You clicked {:?}", boat);
    commands.trigger(UpgradeEvent {
        target: boat
    });
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
    // upgrade_bar: Single<(Entity, &mut Visibility, &mut Text), (With<UpgradeBar>, Without<ProgressBar>)>,
    // sprites: Res<SpriteMap>,
    // progress_bar: Single<(&mut Visibility, &mut Text, &mut BackgroundGradient), With<ProgressBar>>,
) {
    commands.queue(update_upgrade_ui(player_stats.level() + 1, true, true, true));
}

/// say we had zubr at 55 knots, we now upgrade to Espana, we set the input to Espana's max speed, movement system would slow it down
#[derive(Debug, States, Hash, PartialEq, Eq, Clone, Copy)]
pub(crate) enum AfterUpgradeDontClearMoveState {
    NoNeed,
    Sure
}

// these two systems have friends in common::upgrade

// hide the upgradebar and disable click detections (possibly despawn?)
// un-hide progressbar
fn on_choose_upgrade(
    trigger: On<UpgradeEventCommonFinished>,

    mut upgrade_bar: Single<&mut Visibility, With<UpgradeBar>>,
    mut progress_bar: Single<&mut Visibility, (With<ProgressBar>, Without<UpgradeBar>)>,

    boat_q: Single<(&mut Sprite, &mut PlayerStats), With<Controlled>>,
    sprites: Res<SpriteMap>,

    percent: Single<(&mut Text, &mut BackgroundGradient), With<ProgressBar>>,

    mut commands: Commands,

    custom: Single<&CustomTransform, With<Controlled>>,
    mut gradually_decrease: ResMut<NextState<AfterUpgradeDontClearMoveState>>
) {
    // say we had zubr at 55 knots, we now upgrade to Espana, we set the input to Espana's max speed, movement system would slow it down
    if custom.speed > trigger.target.max_speed() {
        // move_input.0.0 = Some(trigger.target.max_speed());
        gradually_decrease.set(AfterUpgradeDontClearMoveState::Sure);
        debug!("gradually_decrease, set move input to {:?}", trigger.target.max_speed());
    } else if custom.speed < - trigger.target.rev_max_speed() {
        // move_input.0.0 = Some(- trigger.target.rev_max_speed());
        gradually_decrease.set(AfterUpgradeDontClearMoveState::Sure);
        debug!("gradually_decrease");
    }

    let (mut sprite, stats) = boat_q.into_inner();
    // debug_assert_eq!(trigger.target.level(), stats.level() + 1); reasoning for ignoring: the updating PlayerStats observer may run before this system
    
    // *stats.level_mut() = trigger.target.level();  // in common
    debug!(upgrade_target = ?trigger.target, "Chose upgrade");
    let multiple_upgrades = 'multiple_upgrade: {
        let DisplayScore::NewLevel(max) = stats.display() else {
            // unreachable!("Reached Upgrade when score not enough");  same reasoning as above
            break 'multiple_upgrade false;
        };

        if trigger.target.level() < max {  // multiple upgrades
            debug!(?max, "Multiple updates");

            commands.queue(update_upgrade_ui(
                trigger.target.level() + 1,
                false,
                false,
                false,
            ));
            true
        } else {
            false
        }
    };
    sprite.image = sprites.image();
    sprite.custom_size = Some(trigger.target.render_size());
    let Some(ref mut texture_atlas) = sprite.texture_atlas else {
        panic!()
    };
    texture_atlas.index = sprites.get_index(trigger.target).unwrap_or_else(|| panic!("{:?} is not in the spritesheet", trigger.target));

    if !multiple_upgrades {
        **upgrade_bar = Visibility::Hidden;
        **progress_bar = Visibility::Visible;

        let (mut text, mut background) = percent.into_inner();
        debug!("Updating percent after upgrade to {}", stats.display().unwrap_percent());
        update_percent(stats.display().unwrap_percent(), trigger.target.level(), &mut text, &mut background);
    }
    commands.trigger(UpdateWeaponSelectionBar { target: trigger.target });
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

/* [common/src/primitives.rs:544:9] self = PlayerStats {
    score: 0,
    level: Two,
}
[common/src/primitives.rs:544:9] min = 30

thread 'main' (88288) panicked at common/src/primitives.rs:545:20:
attempt to subtract with overflow
&&
Displaying wrong upgrade text (+1) when logs say it's correct

2026-06-25T19:19:47.261584Z DEBUG client::ui: Chose upgrade upgrade_target=Zubr
2026-06-25T19:19:47.261596Z ERROR common::primitives: self.score=0 min=30
2026-06-25T19:19:47.261607Z ERROR common::primitives: self.score=0 min=30
2026-06-25T19:19:47.261614Z DEBUG client::ui: Updating percent after upgrade to 0
2026-06-25T19:19:47.261620Z ERROR common::primitives: self.score=0 min=30
*/

/// sets
/// - Text and Background Gradient of progress bar to `target` (optional)
/// - hide the progress bar (optional)
/// - make the upgrade selection menu visible (optional)
/// - update upgrade selection menu text to `"Upgrade to level {target}"`
/// - update the upgrade selection menu's sprites
fn update_upgrade_ui(
    target_level: Level,
    hide_progress_bar: bool,
    unhide_upgrade_ui: bool,
    update_progress_bar: bool,
) -> impl Fn(&mut World) {
    move |world| {
        debug!("Showed upgrade of level {target_level}");
        // let player_stats = world.query_filtered::<&PlayerStats, With<Controlled>>().single(world).unwrap();
        let sprites = world.get_resource::<SpriteMap>().unwrap_or_else(|| unreachable!()).clone();
        if hide_progress_bar {
            let mut progress_bar_vis = world.query_filtered::<&mut Visibility, With<ProgressBar>>().single_mut(world).unwrap();
            *progress_bar_vis = Visibility::Hidden;
        }

        if update_progress_bar {
            let (mut text, mut linear_gradient) = world.query_filtered::<(&mut Text, &mut BackgroundGradient), With<ProgressBar>>().single_mut(world).unwrap();
            
            update_percent(0, target_level, &mut text, &mut linear_gradient);
        }

        let mut vis = world.query_filtered::<&mut Visibility, With<UpgradeBar>>().single_mut(world).unwrap();

        if unhide_upgrade_ui {
            *vis = Visibility::Visible;
        }

        let mut text = world.query_filtered::<&mut Text, With<UpgradeText>>().single_mut(world).unwrap();
        text.0 = format!("Upgrade to level {}", target_level);

        let children = world.query_filtered::<&Children, With<UpgradeBar>>()
            .single(world).map(|c| c.to_vec())
            .unwrap_or(vec![]);  // start with empty children
        let entity = world.query_filtered::<Entity, With<UpgradeBar>>().single(world).unwrap();

        let filtered_children = children
            .into_iter()
            .filter(|child| !world.entity(*child).contains::<UpgradeText>())
            .collect::<Vec<_>>();
        for (child, boat) in zip_longest(filtered_children, target_level.avaliable_boats()) {
            match (child, boat) {
                (Some(child), Some(boat)) => {
                    debug!("Updating {child}'s sprite and associated boat");
                    // do stuff that updates sprite
                    let mut entity_world_mut = world.get_entity_mut(child).unwrap();
                    let mut image = entity_world_mut.get_mut::<ImageNode>().unwrap();
                    image.texture_atlas.as_mut().unwrap().index = sprites.get_index(boat).unwrap();

                    let mut node = entity_world_mut.get_mut::<Node>().unwrap();
                    
                    node.width = px(boat.render_size().x / 2.0);
                    node.height =  px(boat.render_size().y / 2.0);

                    let mut upgrade_option = entity_world_mut.get_mut::<UpgradeOption>().unwrap();
                    upgrade_option.0 = boat;
                }
                (Some(excess), None) => {
                    debug!("Hiding excess upgrade selection entity {excess}");
                    world.commands().entity(excess)
                        .insert(Visibility::Hidden);
                }
                (None, Some(boat)) => {
                    // spawn new ones
                    debug!("Spawning new upgrade option");
                    world.commands().entity(entity)
                        .with_children(|parent| {
                            debug!(new_boat_entity = ?boat);
                            spawn_upgrade_ui_boat!(parent, boat, ?sprites = sprites);
                        });
                }
                (None, None) => unreachable!()
            }
        }
    }
}
struct WeaponUiPlugin;

impl Plugin for WeaponUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputFocus>();
        app.add_observer(spawn_weapon_selection_bar)
            .add_systems(Update, switch_weapon_selection_bar_brightness)
            .add_observer(change_weapon);

        app.add_observer(update_selection_bar)
            .add_observer(update_selection_bar_count);
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

#[derive(Event)]
struct UpdateWeaponSelectionBar {
    target: Boat
}
/// we're assuming that it's selected.aval -= 1
#[derive(Event)]
pub(crate) struct UpdateWeaponSelectionBarCount;

#[derive(Debug, Component)]
struct WeaponDataText;

const FONT_SIZE: FontSize = FontSize::Px(15.0);
const FONT_SIZE_PX: f32 = {
    if let FontSize::Px(p) = FONT_SIZE {
        p
    } else {
        panic!("Expected pixel")
    }
};
const PADDING_TOP: i32 = 10;
const ROW_GAP: i32 = 10;
const PADDING: UiRect = UiRect::all(pixel(10)).with_left(pixel(5)).with_right(pixel(5));

// technically UI stuff
fn spawn_weapon_selection_bar(
    trigger: On<Add, WeaponCounter>,
    weapon_counter: Single<(Entity, &WeaponCounter), With<Controlled>>,
    mut commands: Commands,
    sprites: Res<SpriteUiMap>,
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
    let Some(height) = weapon_counter.weapons.keys().map(|this| {
        sprites.get_size(*this).unwrap().height()
    }).max() else {
        return;  // hm
    };
    let height = get_weapon_selection_bar_height(height);

    base_entity.with_children(|base| {
        for &(weapon, data) in weapon_counter.weapons.iter() {
            let settings = WeaponSelectionSettings {
                height,
                selected_weapon: weapon_counter.selected_weapon.unwrap(),
                weapon,
                data,
                sprites: &sprites,
            };
            base.spawn(weapon_selection_bundle(settings));
        }
    });
}

struct WeaponSelectionSettings<'a> {
    height: u32,
    selected_weapon: Weapon,
    weapon: Weapon,
    data: WeaponData,
    sprites: &'a SpriteUiMap
}

fn weapon_selection_bundle(settings: WeaponSelectionSettings) -> impl Bundle {
    let weapon = settings.weapon;
    (
        // box
        Node {
            height: px(settings.height),  // dyn
            padding: PADDING,
            flex_direction: FlexDirection::Column,  // image and text vertical
            row_gap: px(ROW_GAP),  // so text doesn't get too close to image
            ..default()
        },
        BackgroundColor(if settings.selected_weapon == weapon {  // dyn
            PRESSED_BACKGROUND
        } else {
            NORMAL_BACKGROUND
        }),
        Button,
        WeaponSelectionIndividualBox(weapon),  // dyn
        children![(
            // image
            ImageNode {
                image: settings.sprites.image(),
                texture_atlas: settings.sprites.get(weapon),  // dyn
                color: NORMAL,
                ..default()
            },
            Node {
                position_type: PositionType::Relative,
                width: px(settings.sprites.get_size(weapon).unwrap().width()),  // dyn
                height: px(settings.sprites.get_size(weapon).unwrap().height()),  // dyn
                ..default()
            },
            WeaponSelectionIndividualImage
        ),
        (
            // avaliable/max text
            Text::new(format!("{}/{}", settings.data.avaliable, settings.data.max)),    // dyn ????? lets just refill weapon on upgrade for now
            TextFont {
                font: FontSource::SansSerif,
                font_size: FONT_SIZE,
                style: FontStyle::Normal,
                ..default()
            },
            TextColor(if settings.selected_weapon == weapon {  // dyn
                TEXT_SELECTED
            } else {
                TEXT_LIGHT
            }),
            TextLayout {
                justify: Justify::Right,
                ..default()
            },
            WeaponDataText
        )]
    )
}

const fn get_weapon_selection_bar_height(largest_height: u32) -> u32 {
    largest_height + PADDING_TOP as u32 * 2 + FONT_SIZE_PX as u32 + ROW_GAP as u32
}

#[derive(Debug, QueryData)]
#[query_data(mutable)]
struct WeaponSelectionIndividualData<'a> {
    node: &'a mut Node,
    background: &'a mut BackgroundColor,
    identifier: &'a mut WeaponSelectionIndividualBox,
    children: &'a Children
}
#[derive(Debug, QueryData)]
#[query_data(mutable)]
struct WeaponSelectionIndividualImageData<'a> {
    node: &'a mut Node,
    image: &'a mut ImageNode,
}
#[derive(Debug, QueryData)]
#[query_data(mutable)]
struct WeaponSelectionIndividualTextData<'a> {
    text: &'a mut Text
}
/// on upgrade
fn update_selection_bar(
    trigger: On<UpdateWeaponSelectionBar>,
    sprites: Res<SpriteUiMap>,
    boat_query: Single<(&Boat, &WeaponCounter), With<Controlled>>,

    base_entity: Single<(Entity, &Children), With<WeaponSelection>>,
    mut individual_weapon: Query<WeaponSelectionIndividualData, With<WeaponSelectionIndividualBox>>,
    mut images: Query<WeaponSelectionIndividualImageData, (With<WeaponSelectionIndividualImage>, Without<WeaponSelectionIndividualBox>)>,
    mut texts: Query<WeaponSelectionIndividualTextData, With<WeaponDataText>>,
    mut commands: Commands
) {
    let (boat, weapon_counter) = boat_query.into_inner();
    assert_eq!(*boat, trigger.target); // somehow re-run the observer later...

    
    let Some(height) = weapon_counter.weapons.keys().map(|this| {
        sprites.get_size(*this).unwrap().height()
    }).max() else {
        return;  // hm
    };
    let height = get_weapon_selection_bar_height(height);
    
    let (base, children) = base_entity.into_inner();
    // dbg!(&weapon_counter);
    for (child, weapon) in zip_longest(children, weapon_counter.weapons.iter().copied()) {
        match (child, weapon) {
            (Some(child), Some((weapon, data))) => {
                let mut individual_weapon = individual_weapon.get_mut(*child).unwrap();
                individual_weapon.node.height = px(height);
                // may run into issues about wapon counter not updated
                if weapon_counter.selected_weapon == Some(weapon) {
                    individual_weapon.background.0 = PRESSED_BACKGROUND;
                } else if individual_weapon.background.0 == PRESSED_BACKGROUND {
                    individual_weapon.background.0 = NORMAL_BACKGROUND
                }
                individual_weapon.identifier.0 = weapon;

                {
                    let mut image = get_mut!(individual_weapon.children, images).unwrap();
                    if let Some(atlas) = &mut image.image.texture_atlas {
                        atlas.index = sprites.get_index(weapon).unwrap();
                    } else {
                        warn!("Expected a texture atlas");
                        image.image.texture_atlas = Some(sprites.get(weapon).unwrap());
                    }
                    let size = sprites.get_size(weapon).unwrap();
                    image.node.width = px(size.width());
                    image.node.height = px(size.height());
                }
                {
                    let mut text = get_mut!(individual_weapon.children, texts).unwrap();
                    *text.text = Text::new(format!("{}/{}", data.avaliable, data.max));
                }
            }
            (Some(excess), None) => {
                commands.entity(*excess)
                    .despawn();
            }
            (None, Some((weapon, data))) => {
                let settings = WeaponSelectionSettings {
                    height,
                    selected_weapon: weapon_counter.selected_weapon.unwrap(),
                    weapon,
                    data,
                    sprites: &sprites,
                };
                commands.entity(base)
                    .with_child(weapon_selection_bundle(settings));
            }
            (None, None) => unreachable!()
        }
    }
}

fn update_selection_bar_count(
    _trigger: On<UpdateWeaponSelectionBarCount>,
    weapon_counter: Single<&WeaponCounter, With<Controlled>>,
    container: Query<(&Children, &WeaponSelectionIndividualBox)>,
    mut count: Query<&mut Text, With<WeaponDataText>>
) -> Result {  // not determistic since we can't assume that this would always run before the updating weapon counter system
    let Some(selected) = weapon_counter.selected_weapon else {
        return Err("No selected weapon, should be handled in firing weapon".into())
    };
    let Some((children, _)) = container.into_iter().find(|(_, c)| c.0 == selected) else {
        dbg!(container.iter().collect::<Vec<_>>());
        dbg!(selected);
        return Err("No weapon selection of the currently selected weapon".into())
    };
    let mut text = get_mut!(children, count).unwrap();

    let mut iter = text.0.split('/');
    let (aval, max) = (iter.next().unwrap().parse::<u16>()?, iter.next().unwrap().parse::<u16>()?);
    debug_assert_eq!(iter.next(), None);

    text.0 = format!("{}/{}", aval - 1, max);

    Ok(())
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
                input_focus.set(entity, bevy::input_focus::FocusCause::Pressed);
                commands.trigger(ChangeWeapon { target: *weapon });
                // colouring done in fn change_weapon to avoid ambiguity
                button.set_changed();
            }
            Interaction::Hovered => {
                if background.0 == PRESSED_BACKGROUND {
                    continue;  // already pressing
                }
                input_focus.set(entity, bevy::input_focus::FocusCause::Navigated);
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
        app.add_systems(Startup, Self::spawn_dbg_gui)
            .add_systems(Update, Self::update_dbg_gui);
    }
}


#[derive(Component)]
struct DbgText;
#[allow(dead_code)]
impl DbgPlugin {

    fn on_change(s: Single<&PlayerStats, Changed<PlayerStats>>) {
        info!(
            "Stats: {:?}", s.into_inner()
        )
    }
fn spawn_dbg_gui(mut commands: Commands) {
    commands.spawn((
        Text::new("RotateInput: None\nSpeedInput: None\nState: Stopped\nPosition: None\nAltitude: None\nRotation: None\nSpeed: None\nScore: None"),
        TextFont {
            font: FontSource::Monospace,
            font_size: FONT_SIZE,
            ..default()
        },
        Node {
            left: Val::Vw(75.0),
            ..default()
        },
        ZIndex(1),
        DbgText
    ));
}
fn update_dbg_gui(
    mut text: Single<&mut Text, With<DbgText>>,
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