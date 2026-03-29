//! currently, there are no differentiation between a Ship and a Submarine
//!
//! be mindful of [`Boat::transform`] and [`Transform`] of the [`Boat`] needs to be kept in sync

use std::f32::consts::PI;
use std::time::Duration;

use bevy::color::palettes::css::*;
use bevy::input::keyboard::Key;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

use crate::CIRCLE_HUD;
use crate::DEFAULT_MAX_TURN_DEG;
use crate::DEFAULT_SPRITE_SHRINK;
use crate::DIVING_OVERLAY;
use crate::MainCamera;
use crate::OCEAN_FLOOR;
use crate::WATER_SURFACE;
use crate::collision::out_of_bounds;
use crate::primitives::*;
use crate::shaders::DivingOverlay;
use crate::util::{
    add_circle_hud, calculate_diving_overlay, calculate_from_proportion, eq, get_rotate_radian,
    move_with_rotation,
};
use crate::weapons::SpawnWeaponMessage;
use crate::weapons::Weapon;
use crate::world::WorldSize;

/// filters a [`Query`] that has the last `QueryData` as Boat so that it only contains queries with BoatOwner as Player
macro_rules! filter_player {
    ($query:expr) => {
        $query
            .iter()
            .filter(|(.., boat)| boat.owner == BoatOwner::Player)
    };
    ($query:expr, 1) => {
        $query
            .iter()
            .filter(|(.., boat)| boat.owner == BoatOwner::Player)
            .last()
    };
}

/// client / server
pub struct BoatPlugin;

impl Plugin for BoatPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<BoatState>()
            .init_state::<DivingStatus>()
            .add_message::<FireWeapon>()
            .init_resource::<PlayerScore>()
            .add_systems(Startup, startup)
            .add_systems(Startup, spawn_diving_overlay.after(crate::setup))
            .add_systems(Update, (update_diving_status, update_state))
            .add_systems(
                Update,
                (
                    (rotate_ship, move_ship)
                        .run_if(|state: Res<State<BoatState>>| {
                            matches!(state.get(), BoatState::FreeDir | BoatState::LockedDir)
                        }),
                    ship_to_target.run_if(in_state(BoatState::Released)),
                    update_transform,
                )
                    .chain(),
            )
            .add_systems(Update, (dive, update_diving_overlay))
            .add_systems(Update, fire_weapon)
            .add_systems(PostUpdate, move_camera.after(TransformSystems::Propagate));
    }
}

#[derive(Component, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Boat {
    data: BoatData,
    subkind: SubKind, // should be only one `Boat` in client so no owner
}

#[derive(Component, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum BoatData {
    Yasen,
}

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubKind {
    Submarine,
    SurfaceShip,
}

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq)]
enum BoatOwner {
    Player,
    Bot,
}

const DIVING_OVERLAY_MIN_RADIUS: f32 = 800.0;
const DIVING_OVERLAY_SIZE: Rectangle = Rectangle::from_length(2000.0);
const DIVING_OVERLAY_MAX_RADIUS: f32 = 1000.0;
const DIVING_OVERLAY_MAX_DARKNESS: f32 = 0.6;

/// absolute value of minimum radians that must be reached to reverse the Boat
const MINIMUM_REVERSE: f32 = PI * (2. / 3.);

const TIME_TO_LAUNCH_WEAPON: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, Resource, Default)]
pub(crate) struct PlayerScore(u32);

impl PlayerScore {
    pub(crate) fn add_to_score(&mut self, points: u32) {
        self.0 += points;
    }
    pub(crate) fn get_score(&self) -> u32 {
        self.0
    }
}

/// so a system can manipulate [`Transform`] without state concerns
#[derive(Debug, States, Clone, Copy, Hash, PartialEq, Eq, Default)]
enum DivingStatus {
    Diving,
    Surfacing,
    /// when the submarine shouldn't be moving in altitude
    #[default]
    None,
}

#[derive(Debug, States, Clone, Copy, Hash, PartialEq, Eq, Default)]
enum BoatState {
    /// start state
    #[default]
    Stopped,
    /// potentially fire a weapon
    FiringWeapon(Duration),
    /// locked in a direction (LMB pressed)
    LockedDir,
    /// middle state between `LockedDir` and `Released`, can change direction
    FreeDir,
    /// LMB not pressed
    Released,
}

/// when player releases LMB in the allocated time period, sent by [`BoatState`]
///
/// local message
#[derive(Debug, Message, Clone, Copy, Hash, PartialEq, Eq, Default)]
struct FireWeapon;

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let yasen = BoatData::Yasen;

    let position = vec3(0.0, 0.0, WATER_SURFACE);
    let radius = add_circle_hud(yasen.sprite_size().x / 2.0);
    let sprite = Sprite {
        image: asset_server.load("yasen.png"),
        custom_size: Some(yasen.sprite_size()),
        ..default()
    };
    commands
        .spawn(BoatBundle {
            weapon_counter: WeaponCounter {
                aval_weapons: yasen.get_armanents(),
                selected_weapon: yasen.default_weapon(),
            },
            boat: Boat {
                data: BoatData::Yasen,
                subkind: SubKind::Submarine,
                owner: BoatOwner::Player,
            },
            sprite,
            transform: Transform::from_translation(position),
            custom_transform: CustomTransform {
                position: Position(position.xy()),
                rotation: Radian::from_deg(90.0),
                ..default()
            },
            ..Default::default()
        })
        .with_children(
            |parent: &mut bevy::ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>| {
                parent.spawn((
                    MeshBundle {
                        mesh: Mesh2d(meshes.add(Circle::new(radius).to_ring(3.0))),
                        materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
                    },
                    Transform::from_xyz(0.0, 0.0, CIRCLE_HUD),
                    CircleHud {
                        radius,
                        center: position.xy(),
                    },
                ));
            },
        );
}

fn spawn_diving_overlay(
    mut commands: Commands,
    mut diving_overlay_material: ResMut<Assets<DivingOverlay>>,
    mut meshes: ResMut<Assets<Mesh>>,
    camera: Single<Entity, With<MainCamera>>,
) {
    if let Ok(mut camera) = commands.get_entity(*camera) {
        camera.with_children(|parent| {
            parent.spawn((
                Transform::from_xyz(0.0, 0.0, DIVING_OVERLAY),
                MeshMaterial2d(diving_overlay_material.add(DivingOverlay {
                    radius: DIVING_OVERLAY_MAX_RADIUS,
                    player_pos: vec2(0.0, 0.0), // TODO
                    darkness: 0.0,
                    ..default()
                })),
                Mesh2d(meshes.add(DIVING_OVERLAY_SIZE)),
                DivingOverlayIdentifier,
            ));
        });
    }
}

/// helper struct for accessing the [`Boat`]'s circle HUD
#[derive(Debug, Component, Copy, Clone)]
pub(crate) struct CircleHud {
    pub radius: f32,
    pub center: Vec2,
}

impl CircleHud {
    /// whether `point` is in the Circle HUD
    pub(crate) fn contains(&self, point: Vec2) -> bool {
        point.distance_squared(self.center) < self.radius.powi(2)
    }
    /// whether a point is at HUD's center
    ///
    /// adjusted for decimal-point precision
    pub(crate) fn at_center(&self, point: Vec2, decimal_point: DecimalPoint) -> bool {
        let x_diff = (point.x - self.center.x).abs();
        let y_diff = (point.y - self.center.y).abs();

        x_diff < decimal_point.to_f32() && y_diff < decimal_point.to_f32()
    }
}

#[derive(Debug, Component, Clone, Copy)]
struct DivingOverlayIdentifier;

fn move_camera(
    mut camera: Single<&mut Transform, With<MainCamera>>,
    ship_pos: Query<&CustomTransform, With<Boat>>,
) {
    // currently ignores possibility of multiple ships
    let Some(ship) = ship_pos.iter().last() else {
        return;
    };

    if ship.position.0 != camera.translation.xy() {
        camera.translation = ship.position.0.extend(WATER_SURFACE);
    }
}

/// note that states are updated between frames, so the position of this system doesn't matter
fn update_state(
    current_state: Res<State<BoatState>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut setter: ResMut<NextState<BoatState>>,
    mut fire_weapon: MessageWriter<FireWeapon>,
    time: Res<Time>,
) {
    match current_state.get() {
        BoatState::Stopped => {
            if buttons.just_pressed(MouseButton::Left) {
                // may not be correct
                setter.set(BoatState::FreeDir);
            }
        }
        BoatState::LockedDir => {
            if buttons.just_released(MouseButton::Left) {
                setter.set(BoatState::Released);
            }
        }
        BoatState::FreeDir => {
            // allow 1 frame in freedir
            setter.set(BoatState::LockedDir);
        }
        BoatState::Released => {
            if buttons.just_pressed(MouseButton::Left) {
                setter.set(BoatState::FiringWeapon(Duration::ZERO));
            }
        }
        BoatState::FiringWeapon(elapsed) => {
            let duration = *elapsed + time.delta();

            if duration > TIME_TO_LAUNCH_WEAPON {
                setter.set(BoatState::FreeDir);
            } else if buttons.just_released(MouseButton::Left) {
                fire_weapon.write(FireWeapon);
                setter.set(BoatState::Released);
            } else {
                setter.set(BoatState::FiringWeapon(duration));
            }
        }
    }
}

/// handle rotation
fn rotate_ship(
    mut transforms: Query<(&Transform, &mut CustomTransform, &mut TargetRotation, &Boat)>,
    state: Res<State<BoatState>>,
    cursor_pos: Res<CursorPos>,
) {
    let state = *state.get();

    for (transform, mut custom_transform, mut target_rotation, boat) in transforms.iter_mut() {
        let raw_moved = get_rotate_radian(transform.translation.xy(), cursor_pos.0); // diff from radian 0
        let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);
        let mut target_move = raw_moved;

        let moved = {
            // radians to move from current rotation
            let mut moved_from_current = (raw_moved - current_rotation).normalize();

            // -- adjust for reversed ---
            if moved_from_current.abs() > MINIMUM_REVERSE && state == BoatState::FreeDir {
                // reversing
                custom_transform.reversed = true;
                moved_from_current = moved_from_current.flip();
                target_move = target_move.flip()
            } else if moved_from_current.abs() <= MINIMUM_REVERSE
                && custom_transform.reversed
                && state == BoatState::FreeDir
            {
                // going forwards
                custom_transform.reversed = false;
            } else if custom_transform.reversed {
                // unable to go forward, haven't released key yet
                moved_from_current = moved_from_current.flip();
                target_move = target_move.flip()
            }

            moved_from_current
        };

        // turning degree bigger than maximum
        if moved.abs() > boat.data.max_turn().to_radians() {
            let ship_max_turn = boat.data.max_turn().to_radians();
            if moved > 0.0 {
                custom_transform.rotate_local_z(ship_max_turn.to_radian_unchecked());
            } else if moved < 0.0 {
                custom_transform.rotate_local_z(-ship_max_turn.to_radian_unchecked());
            }
        } else {
            // normal
            custom_transform.rotate_local_z(moved.to_radian_unchecked());
        }

        target_rotation.0 = Some(target_move);
    }
}

/// handle moving
fn move_ship(
    mut datas: Query<(&Transform, &mut CustomTransform, &mut TargetSpeed, &Boat)>,
    cursor_pos: Res<CursorPos>,
) {
    for (transform, mut custom_transform, mut target_speed, boat) in datas.iter_mut() {
        let cursor_distance = cursor_pos.0.distance(transform.translation.xy());
        let max_speed = if custom_transform.reversed {
            -boat.data.rev_max_speed().get_raw()
        } else {
            boat.data.max_speed().get_raw()
        };

        let speed = calculate_from_proportion(
            cursor_distance,
            add_circle_hud(boat.data.sprite_size().x / 2.0),
            max_speed,
            boat.data.sprite_size().x / 2.0,
        );

        target_speed.0 = Speed::from_raw(speed);

        // adjust for acceleration
        let speed_diff = speed - custom_transform.speed.get_raw();
        let acceleration = boat.data.acceleration();

        if speed_diff > acceleration.get_raw() {
            // accelerating too much forwards
            custom_transform.speed.add_raw(acceleration.get_raw());
        } else if speed_diff < -acceleration.get_raw() {
            // accelerating too much backwards
            custom_transform.speed.subtract_raw(acceleration.get_raw());
        }
        // not exceeding acceleration
        else if speed_diff.abs() > 0.1 {
            custom_transform.speed.overwrite_with_raw(speed);
        }
    }
}

// note that we're accepting Query instead of Single for ship everywhere

/// remember the last move angle and rotate toward it when button not pressed
fn ship_to_target(
    mut ships: Query<(
        &Transform,
        &mut CustomTransform,
        &TargetRotation,
        &TargetSpeed,
        &Boat,
    )>,
) {
    for (transform, mut custom_transform, target_rotation, target_speed, boat) in ships.iter_mut() {
        // ------ rotation
        let Some(target_rotation) = target_rotation.0 else {
            continue;
        };

        let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

        let moved = (target_rotation - current_rotation).normalize();

        let ship_max_turn = boat.data.max_turn().to_radians();
        if moved.abs() > ship_max_turn {
            if moved > 0.0 {
                custom_transform.rotate_local_z(ship_max_turn.to_radian_unchecked());
            } else if moved < 0.0 {
                custom_transform.rotate_local_z(-ship_max_turn.to_radian_unchecked());
            }
        } else {
            custom_transform.rotate_local_z(moved.to_radian_unchecked());
        }
        // ------ speed
        let speed_diff = target_speed.get_raw() - custom_transform.speed.get_raw();
        let acceleration = boat.data.acceleration();
        if speed_diff > acceleration.get_raw() {
            custom_transform.speed.add_raw(acceleration.get_raw());
        } else if speed_diff < -acceleration.get_raw() {
            custom_transform.speed.subtract_raw(acceleration.get_raw());
        } else {
            custom_transform
                .speed
                .overwrite_with_raw(target_speed.get_raw());
        }
    }
}

/// updates [`Boat`]'s [`Transform`] according to its [`CustomTransform`]
fn update_transform(
    mut transform_ship: Query<
        (
            &mut Transform,
            &mut CustomTransform,
            &Children,
            &Sprite,
            &mut OutOfBound,
        ),
        With<Boat>,
    >,
    mut circle_huds: Query<&mut CircleHud>,
    world_size: Single<&WorldSize>,
) {
    for (mut transform, mut custom, children, sprite, mut out_of_bound) in transform_ship.iter_mut()
    {
        let Some(custom_size) = sprite.custom_size else {
            continue;
        };

        let mut translation = custom.position.to_vec3(transform.translation.z);

        translation += move_with_rotation(transform.rotation, custom.speed.get_raw()); // ignores frame lagging temporary

        if out_of_bounds(
            &world_size,
            MkRect {
                center: translation.xy(),
                dimensions: custom_size.into(),
            },
            custom.rotation.to_quat(),
        ) {
            custom.position.0 = transform.translation.truncate();
            out_of_bound.0 = true;
            continue;
        } else if out_of_bound.0 {
            out_of_bound.0 = false;
        }

        let target = Transform {
            translation,
            rotation: custom.rotation.to_quat(),
            scale: Vec3::ONE,
        };
        *transform = target;

        // sync position
        custom.position = Position(translation.xy());

        for child in children {
            if let Ok(mut hud) = circle_huds.get_mut(*child) {
                hud.center = translation.xy();
                break;
            }
        }

        // println!("Speed: {} knots", custom.speed.get_knots());
    }
}

fn dive(mut ships: Query<(&mut Transform, &Boat)>, diving_status: Res<State<DivingStatus>>) {
    let (mut transform, boat) = ships
        .iter_mut()
        .find(|(.., boat)| matches!(boat.owner, BoatOwner::Player))
        .expect("Player died?");

    if boat.subkind != SubKind::Submarine {
        return;
    }

    match diving_status.get() {
        DivingStatus::Diving => {
            transform.decrease_with_limit(boat.data.diving_speed().get_raw(), OCEAN_FLOOR)
        }
        DivingStatus::Surfacing => {
            transform.increase_with_limit(boat.data.diving_speed().get_raw(), OCEAN_FLOOR)
        }
        DivingStatus::None => {}
    }
}

fn update_diving_status(
    mut setter: ResMut<NextState<DivingStatus>>,
    getter: Res<State<DivingStatus>>,
    buttons: Res<ButtonInput<Key>>,
    transforms: Query<(&Transform, &Boat)>,
) {
    let (transform, _) = filter_player!(transforms, 1).unwrap();
    let mut target = *getter.get();

    match target {
        DivingStatus::None => (),
        DivingStatus::Surfacing => {
            if transform.reached(WATER_SURFACE, DecimalPoint::Three) {
                target = DivingStatus::None;
            }
        }
        DivingStatus::Diving => {
            if transform.reached(OCEAN_FLOOR, DecimalPoint::Three) {
                target = DivingStatus::None;
            }
        }
    }

    if buttons.just_pressed(Key::Character("r".into()))
        || buttons.just_pressed(Key::Character("R".into()))
    {
        match target {
            DivingStatus::None => {
                if eq(transform.translation.z, 0.0, DecimalPoint::Three) {
                    target = DivingStatus::Diving
                } else {
                    target = DivingStatus::Surfacing;
                }
            }
            DivingStatus::Surfacing => target = DivingStatus::Diving,
            DivingStatus::Diving => target = DivingStatus::Surfacing,
        }
    }

    setter.set(target);
}

fn update_diving_overlay(
    ship_pos: Query<&CustomTransform, With<Boat>>,
    transforms: Query<&Transform, With<Boat>>,
    mut diving_overlay_material: ResMut<Assets<DivingOverlay>>,
    id: Single<&MeshMaterial2d<DivingOverlay>>,
) {
    // currently ignores possibility of multiple ships
    let Some(ship) = ship_pos.iter().last() else {
        return;
    };
    let Some(ship_transform) = transforms.iter().last() else {
        return;
    };

    if let Some(diving_material) = diving_overlay_material.get_mut(*id) {
        diving_material.player_pos = ship.position.0;
        (diving_material.radius, diving_material.darkness) = calculate_diving_overlay(
            ship_transform.translation.z,
            OCEAN_FLOOR,
            DIVING_OVERLAY_MIN_RADIUS,
            DIVING_OVERLAY_MAX_RADIUS,
            DIVING_OVERLAY_MAX_DARKNESS,
        )
    }
}

fn fire_weapon(
    mut receiver: MessageReader<FireWeapon>,
    boats: Query<(&Transform, &Boat)>,
    mut writer: MessageWriter<SpawnWeaponMessage>,
    cursor_pos: Res<CursorPos>,
) {
    let (transform, boat) = filter_player!(boats, 1).unwrap();

    for _ in receiver.read() {
        let fire_angle: f32 = get_rotate_radian(transform.translation.xy(), cursor_pos.0);
        if let Some(weapon) = boat.data.default_weapon() {
            writer.write(SpawnWeaponMessage {
                weapon,
                position: transform.translation.xy(),
                rotation: transform.rotation,
                target_rotation: Quat::from_rotation_z(fire_angle),
            });
        }
    }
}

impl BoatData {
    fn get_armanents(&self) -> Vec<Weapon> {
        match self {
            Self::Yasen => vec![Weapon::Set65],
        }
    }
    fn default_weapon(&self) -> Option<Weapon> {
        match self {
            Self::Yasen => Some(Weapon::Set65),
        }
    }
    fn max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Yasen => 35.0,
        })
    }
    fn rev_max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Yasen => 21.0,
        })
    }
    fn diving_speed(&self) -> Speed {
        Speed::from_raw(match self {
            Self::Yasen => 0.004,
        })
    }
    fn acceleration(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Yasen => 2.0,
        })
    }
    /// raw file size * [`DEFAULT_SPRITE_SHRINK`]
    fn sprite_size(&self) -> Vec2 {
        (match self {
            Self::Yasen => vec2(1024.0, 156.0),
        }) * DEFAULT_SPRITE_SHRINK
    }
    /// max turn in degrees
    fn max_turn(&self) -> f32 {
        DEFAULT_MAX_TURN_DEG
    }
}

#[derive(Bundle, Debug, Clone)]
pub struct BoatBundle {
    /// tranform to update in seperate system
    transform: Transform,
    /// ship's sprite
    sprite: Sprite,
    /// whether reversed, speed etc
    custom_transform: CustomTransform,
    /// where the user's mouse was facing
    mouse_target: TargetRotation,
    /// the target speed of the Boat
    target_speed: TargetSpeed,
    out_of_bound: OutOfBound,

    weapon_counter: WeaponCounter,
    boat: Boat,
}

impl Default for BoatBundle {
    /// Should be overwritten:
    /// - `boat`
    /// - `weapon_counter`
    /// - `max_turn`
    /// - `sprite`
    /// - `transform`
    /// - `custom_transform`
    fn default() -> Self {
        BoatBundle {
            transform: Transform::default(),
            sprite: Sprite::default(),
            custom_transform: CustomTransform::default(),
            out_of_bound: OutOfBound(false),
            mouse_target: TargetRotation::default(),
            target_speed: TargetSpeed::default(),
            weapon_counter: WeaponCounter {
                aval_weapons: vec![],
                selected_weapon: None,
            },
            boat: Boat {
                data: BoatData::Yasen,
                subkind: SubKind::SurfaceShip,
                owner: BoatOwner::Player,
            },
        }
    }
}
