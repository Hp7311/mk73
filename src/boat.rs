//! currently, there are no differentiation between a Ship and a Submarine
//!
//! be mindful of [`Boat::transform`] and [`Transform`] of the [`Boat`] needs to be kept in sync

// doc outdated

use std::f32::consts::PI;
use std::time::Duration;

use bevy::color::palettes::css::*;
use bevy::input::keyboard::Key;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::CIRCLE_HUD;
use crate::DEFAULT_SPRITE_SHRINK;
use crate::DIVING_OVERLAY;
use crate::MainCamera;
use crate::OCEAN_FLOOR;
use crate::WATER_SURFACE;
use crate::collision::out_of_bounds;
use crate::primitives::*;
use crate::shaders::DivingOverlay;
use crate::util::calculate_diving_overlay;
use crate::util::eq;
use crate::util::{
    add_circle_hud, calculate_from_proportion, get_cursor_pos, get_rotate_radian,
    move_with_rotation,
};
use crate::weapons::SpawnWeaponMessage;
use crate::weapons::Weapon;
use crate::world::WorldSize;

pub struct BoatPlugin;

impl Plugin for BoatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup)
            .add_systems(Startup, spawn_diving_overlay.after(crate::setup))
            .init_resource::<PlayerScore>()
            .init_resource::<FiringButtonPressed>()
            .add_systems(Update, (update_ship, update_transform).chain())
            .add_systems(Update, diving)
            .add_systems(Update, update_diving_overlay)
            .add_systems(PostUpdate, move_camera.after(TransformSystems::Propagate));
    }
}

#[derive(Component, Debug, Copy, Clone)]
struct Boat {
    data: BoatData,
    subkind: SubKind,
    owner: BoatOwner
}

#[derive(Component, Debug, Copy, Clone)]
enum BoatData {
    Yasen
}

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq)]
enum SubKind {
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

#[derive(Debug, Clone, Copy, Resource, Default)]
struct FiringButtonPressed {
    firing_angle: Option<f32>,
    time_since_key_down: Duration
}

#[derive(Debug, Component, Clone, Copy, Default)]
enum DivingStatus {
    Diving,
    Surfacing,
    /// when the submarine shouldn't be moving in altitude
    #[default]
    None
}

// TODO modify all things that take Speed related to take Yasen seeing as it's a method on it
fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let yasen = BoatData::Yasen;

    let position = vec2(0.0, 0.0);
    let radius = add_circle_hud(yasen.sprite_size().x / 2.0);
    let sprite = Sprite {
        image: asset_server.load("yasen.png"),
        custom_size: Some(yasen.sprite_size()),
        ..default()
    };
    commands
        .spawn((
            BoatBundle::new(
                yasen.max_speed(),
                yasen.rev_max_speed(),
                yasen.diving_speed(),
                yasen.acceleration(),
                position,
                sprite,
            ),
            WeaponCounter {
                aval_weapons: yasen.get_armanents(),
                selected_weapon: yasen.default_weapon()
            },
            DivingStatus::default(),
            Boat {
                data: BoatData::Yasen,
                subkind: SubKind::Submarine,
                owner: BoatOwner::Player
            }
        ))
        .with_children(|parent: &mut bevy::ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>| {
            parent.spawn((
                CircleHudBundle {
                    mesh: Mesh2d(meshes.add(Circle::new(radius).to_ring(3.0))),
                    materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
                },
                Transform::from_xyz(0.0, 0.0, CIRCLE_HUD),
                CircleHud {
                    radius,
                    center: position,
                },
            ));
        });
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
                    player_pos: vec2(0.0, 0.0), // assume
                    darkness: 0.0,
                })),
                Mesh2d(meshes.add(DIVING_OVERLAY_SIZE)),
                DivingOverlayIdentifier,
            ));
        });
    }
}

/// helper struct for accessing the [`Boat`](crate::ship::Boat)'s circle HUD
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

#[derive(Debug, Component, Clone)]
pub(crate) struct WeaponCounter {
    aval_weapons: Vec<Weapon>,  // FIXME and maybe HashMap<Weapon, u16>
    selected_weapon: Option<Weapon>  // potential terry fox
}

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

/// modifys [`Transform`] of [`Boat`]
fn update_ship(
    buttons: Res<ButtonInput<MouseButton>>,
    mut firing_button: ResMut<FiringButtonPressed>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut queries: ParamSet<(
        Query<
            (
                &Transform,
                &mut CustomTransform,
                &Radian,
                &mut TargetRotation,
                &LmbReleased,
            ),
            With<Boat>,
        >,
        Query<
            (
                &Transform,
                &mut CustomTransform,
                &Radian,
                &TargetRotation,
                &TargetSpeed,
                &Acceleration,
            ),
            With<Boat>,
        >,
        Query<
            (
                &Transform,
                &mut CustomTransform,
                &Radius,
                &MaxSpeed,
                &ReverseSpeed,
                &Acceleration,
                &mut TargetSpeed,
            ),
            With<Boat>,
        >,
        Query<&mut LmbReleased, With<Boat>>,
        Query<(&Transform, &Boat)>
    )>,
    mut spawn_weapon_writer: MessageWriter<SpawnWeaponMessage>,
    time: Res<Time>
) {
    // assume single ship
    let datas = queries.p4();
    let (transform, boat) = datas.single().unwrap();
    
    if let Some(cursor_pos) = get_cursor_pos(&window, &camera) {

        // fire a weapon if pressed down time smaller than const specified
        if firing_button.pressed() {
            firing_button.time_since_key_down += time.delta();
        }
        if buttons.just_pressed(MouseButton::Left) {
            match firing_button.firing_angle {
                Some(_) => unreachable!(),
                None => {
                    let firing_angle = get_rotate_radian(cursor_pos, transform.translation.xy());
                    firing_button.firing_angle = Some(firing_angle);
                }
            }
        } else if firing_button.time_since_key_down > TIME_TO_LAUNCH_WEAPON {
            if firing_button.pressed() {
                firing_button.reset();
            }
        }
        
        if buttons.just_released(MouseButton::Left) && let Some(firing_angle) = firing_button.firing_angle {
            // --- fires a weapon
            if let Some(weapon) = boat.data.default_weapon() {
                spawn_weapon_writer.write(SpawnWeaponMessage {
                    weapon,
                    position: transform.translation.xy(),
                    rotation: transform.rotation,
                    target_rotation: Quat::from_rotation_z(firing_angle)
                });
                return;  //TODO messy state machine with duplication
            }
            
        } else if buttons.just_released(MouseButton::Left) {
            firing_button.reset();
        }
        
        if firing_button.time_since_key_down < TIME_TO_LAUNCH_WEAPON && firing_button.pressed() {
            // don't move when unclear
            return;
        }
    }

    if let Some(cursor_pos) = get_cursor_pos(&window, &camera)
        && buttons.pressed(MouseButton::Left)
    {
        rotate_ship(&mut queries.p0(), cursor_pos);
        move_ship(&mut queries.p2(), cursor_pos);
    } else {
        ship_to_target(&mut queries.p1());
    }

    if get_cursor_pos(&window, &camera).is_some() && buttons.just_released(MouseButton::Left) {
        for mut released in queries.p3() {
            released.0 = true;
        }
    } else if buttons.pressed(MouseButton::Left) {
        for mut released in queries.p3() {
            released.0 = false;
        }
    }
}

/// handle rotation
fn rotate_ship(
    transforms: &mut Query<
        (
            &Transform,
            &mut CustomTransform,
            &Radian,
            &mut TargetRotation,
            &LmbReleased,
        ),
        With<Boat>,
    >,
    cursor_pos: Vec2,
) {
    for (transform, mut custom_transform, max_turn, mut target_rotation, released) in
        transforms.iter_mut()
    {
        let raw_moved = get_rotate_radian(cursor_pos, transform.translation.xy()); // diff from radian 0
        let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);
        let mut target_move = raw_moved;

        let moved = {
            // radians to move from current rotation
            let mut moved_from_current = (raw_moved.to_degrees() - current_rotation.to_degrees())
                .to_radians()
                .trim();

            // -- adjust for reversed ---
            if moved_from_current.abs() > MINIMUM_REVERSE && released.0 {
                // mouse in area and LMB released
                custom_transform.reversed = true;
                moved_from_current = moved_from_current.flip();
                target_move = target_move.flip()
            } else if custom_transform.reversed && released.0 {
                // already reversing but LMB released
                custom_transform.reversed = false;
            } else if custom_transform.reversed {
                // unable to go forward, haven't released key yet
                moved_from_current = moved_from_current.flip();
                target_move = target_move.flip()
            }

            moved_from_current
        };

        // turning degree bigger than maximum
        if moved.abs() > max_turn.0 {
            let ship_max_turn = max_turn.0;
            if moved > 0.0 {
                custom_transform.rotate_local_z(ship_max_turn.to_radian_unchecked());
            } else if moved < 0.0 {
                custom_transform.rotate_local_z(-ship_max_turn.to_radian_unchecked());
            }
        } else {
            // normal
            custom_transform.rotate_local_z(moved.to_radian_unchecked());
        }

        *target_rotation = Some(target_move).into();
    }
}

/// handle moving
fn move_ship(
    datas: &mut Query<
        (
            &Transform,
            &mut CustomTransform,
            &Radius,
            &MaxSpeed,
            &ReverseSpeed,
            &Acceleration,
            &mut TargetSpeed,
        ),
        With<Boat>,
    >,
    cursor_pos: Vec2,
) {
    for (
        transform,
        mut custom_transform,
        radius,
        max_speed,
        reverse_speed,
        acceleration,
        mut target_speed,
    ) in datas.iter_mut()
    {
        let cursor_distance = cursor_pos.distance(transform.translation.xy());
        let max_speed = if custom_transform.reversed {
            -reverse_speed.0.get_raw()
        } else {
            max_speed.0.get_raw()
        };

        let speed = calculate_from_proportion(
            cursor_distance,
            add_circle_hud(radius.0),
            max_speed,
            radius.0,
        );

        target_speed.0 = Speed::from_raw(speed);

        // adjust for acceleration
        let speed_diff = speed - custom_transform.speed.get_raw();

        if speed_diff > acceleration.0.get_raw() {
            // accelerating too much forwards
            custom_transform.speed.add_raw(acceleration.0.get_raw());
        } else if speed_diff < -acceleration.0.get_raw() {
            // accelerating too much backwards
            custom_transform
                .speed
                .subtract_raw(acceleration.0.get_raw());
        }
        // not exceeding acceleration
        else if speed_diff.abs() > 0.1 {
            custom_transform.speed.overwrite_with_raw(speed);
        }
    }
}

// note that we're accepting Query instead of Single for ship everywhere
// and not descriminating Bot/Player

/// remember the last move angle and rotate toward it when button not pressed
fn ship_to_target(
    ships: &mut Query<
        (
            &Transform,
            &mut CustomTransform,
            &Radian,
            &TargetRotation,
            &TargetSpeed,
            &Acceleration,
        ),
        With<Boat>,
    >,
) {
    for (transform, mut custom_transform, max_turn, target_rotation, target_speed, acceleration) in
        ships
    {
        // ------ rotation
        let Some(target_rotation) = target_rotation.0 else {
            continue;
        };

        let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

        let moved = (target_rotation.to_degrees() - current_rotation.to_degrees())
            .to_radians()
            .trim();

        if moved.abs() > max_turn.0 {
            let ship_max_turn = max_turn.0;
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
        if speed_diff > acceleration.get_raw() {
            custom_transform.speed.add_raw(acceleration.0.get_raw());
        } else if speed_diff < -acceleration.get_raw() {
            custom_transform
                .speed
                .subtract_raw(acceleration.get_raw());
        } else {
            custom_transform.speed.overwrite_with_raw(target_speed.get_raw());
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

fn diving(
    mut ships: Query<(&mut Transform, &mut DivingStatus, &DivingSpeed, &Boat)>,
    buttons: Res<ButtonInput<Key>>
) {
    let (mut transform, mut diving_status, diving_speed, boat) = ships
        .iter_mut()
        .find(|(.., boat)| matches!(boat.owner, BoatOwner::Player))
        .expect("Player died?");

    if boat.subkind != SubKind::Submarine {
        return;
    }

    if buttons.just_pressed(Key::Character("r".into()))
        || buttons.just_pressed(Key::Character("R".into()))
    {
        match *diving_status {
            DivingStatus::None => {
                if eq(transform.translation.z, 0.0, DecimalPoint::Three) {
                    *diving_status = DivingStatus::Diving;
                } else {
                    *diving_status = DivingStatus::Surfacing;
                }
            },
            DivingStatus::Surfacing => {
                *diving_status = DivingStatus::Diving;
            },
            DivingStatus::Diving => {
                *diving_status = DivingStatus::Surfacing;
            },
        }
    }

    match *diving_status {
        DivingStatus::Diving => transform.decrease_with_limit(diving_speed.0, OCEAN_FLOOR),
        DivingStatus::Surfacing => transform.increase_with_limit(diving_speed.0, OCEAN_FLOOR),
        DivingStatus::None => {}
    }
    
    match *diving_status {
        DivingStatus::Diving => if transform.reached(OCEAN_FLOOR, DecimalPoint::Three) {
            *diving_status = DivingStatus::None;
        },
        DivingStatus::Surfacing => if transform.reached(WATER_SURFACE, DecimalPoint::Three) {
            *diving_status = DivingStatus::None;
        },
        DivingStatus::None => {}
    }
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
            DIVING_OVERLAY_MAX_DARKNESS
        )
    }
}


impl BoatData {
    fn get_armanents(&self) -> Vec<Weapon> {
        match self {
            Self::Yasen => vec![Weapon::Set65]
        }
    }
    fn default_weapon(&self) -> Option<Weapon> {
        match self {
            Self::Yasen => Some(Weapon::Set65)
        }
    }
    fn max_speed(&self) -> f32 {
        match self {
            Self::Yasen => 35.0
        }
    }
    fn rev_max_speed(&self) -> f32 {
        match self {
            Self::Yasen => 21.0
        }
    }
    fn diving_speed(&self) -> f32 {
        match self {
            Self::Yasen => 0.004
        }
    }
    fn acceleration(&self) -> f32 {
        match self {
            Self::Yasen => 0.3
        }
    }
    /// raw file size * [`DEFAULT_SPRITE_SHRINK`]
    fn sprite_size(&self) -> Vec2 {
        ( match self {    
            Self::Yasen => vec2(1024.0, 156.0)
        } ) * DEFAULT_SPRITE_SHRINK
    }
}

impl FiringButtonPressed {
    fn pressed(&self) -> bool {
        self.firing_angle.is_some()
    }
    fn reset(&mut self) {
        *self = Self::default();
    }
}