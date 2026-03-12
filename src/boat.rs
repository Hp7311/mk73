//! currently, there are no differentiation between a Ship and a Submarine
//!
//! be mindful of [`Boat::transform`] and [`Transform`] of the [`Boat`] needs to be kept in sync

// doc outdated

use std::f32::consts::PI;

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
use crate::util::{
    add_circle_hud, calculate_from_proportion, get_cursor_pos, get_rotate_radian,
    move_with_rotation,
};
use crate::world::WorldSize;

pub struct BoatPlugin;

impl Plugin for BoatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup)
            .add_systems(Startup, spawn_diving_overlay.after(crate::setup))
            .insert_resource(PlayerScore(0))
            .add_systems(Update, (update_ship, update_transform).chain())
            .add_systems(Update, diving)
            .add_systems(PostUpdate, move_camera.after(TransformSystems::Propagate));
    }
}

#[derive(Component, Debug, Copy, Clone)]
struct Boat;

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq)]
enum SubKind {
    Submarine,
    SurfaceShip
}

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq)]
enum BoatOwner {
    Player,
    Bot
}

const YASEN_MAX_SPEED: f32 = 35.0; // using HashMap?
const YASEN_BACK_SPEED: f32 = 21.0;
const YASEN_DIVING_SPEED: f32 = 0.1;
const YASEN_ACCELERATION: f32 = 0.3;

const YASEN_RAW_SIZE: Vec2 = vec2(1024.0, 156.0);
/// absolute value of minimum radians that must be reached to reverse the Boat
const MINIMUM_REVERSE: f32 = PI * (2. / 3.);

#[derive(Debug, Clone, Copy, Resource)]
pub(crate) struct PlayerScore(u32);

impl PlayerScore {
    pub(crate) fn add_to_score(&mut self, points: u32) {
        self.0 += points;
    }
    pub(crate) fn get_score(&self) -> u32 {
        self.0
    }
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>
) {
    let position = vec2(0.0, 0.0);
    let radius = add_circle_hud(YASEN_RAW_SIZE.x * DEFAULT_SPRITE_SHRINK / 2.0);
    let sprite = Sprite {
        image: asset_server.load("yasen.png"),
        custom_size: Some(YASEN_RAW_SIZE * DEFAULT_SPRITE_SHRINK),
        ..default()
    };
    commands
        .spawn((
            BoatBundle::new(
                YASEN_MAX_SPEED,
                YASEN_BACK_SPEED,
                YASEN_DIVING_SPEED,
                YASEN_ACCELERATION,
                position,
                sprite
            ),
            SubKind::Submarine,
            BoatOwner::Player,
            Boat
        ))
        .with_children(|parent| {
            parent.spawn((
                CircleHudBundle {
                    mesh: Mesh2d(meshes.add(Circle::new(radius).to_ring(3.0))),
                    materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
                },
                Transform::from_translation(vec3(0.0, 0.0, CIRCLE_HUD)),
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
    camera: Single<Entity, With<MainCamera>>
) {
    if let Ok(mut camera) = commands.get_entity(*camera) {
        camera.with_children(|parent| {
            parent.spawn((
                Transform::from_xyz(0.0, 0.0, DIVING_OVERLAY),
                MeshMaterial2d(diving_overlay_material.add(DivingOverlay {
                    radius: 500.0,
                    player_pos: vec2(0.0, 0.0),  // assume
                    darkness: 0.0
                })),
                Mesh2d(meshes.add(Rectangle::from_length(2000.0))),
                DivingOverlayIdentifier
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

        let max_distance = match decimal_point {
            DecimalPoint::Zero => 1.0,
            DecimalPoint::One => 0.1,
            DecimalPoint::Two => 0.01,
            DecimalPoint::Three => 0.001,
        };

        x_diff < max_distance && y_diff < max_distance
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

/// modifys [`Transform`] of [`Boat`]
fn update_ship(
    buttons: Res<ButtonInput<MouseButton>>,
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
    )>,
) {
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
            &LmbReleased
        ),
        With<Boat>,
    >,
    cursor_pos: Vec2,
) {
    for (
        transform,
        mut custom_transform,
        max_turn,
        mut target_rotation,
        released
    ) in transforms.iter_mut()
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
            if moved_from_current.abs() > MINIMUM_REVERSE && released.0 {  // mouse in area and LMB released
                custom_transform.reversed = true;
                moved_from_current = moved_from_current.flip();
                target_move = target_move.flip()
            }
            else if custom_transform.reversed && released.0 {  // already reversing but LMB released
                custom_transform.reversed = false;
            } else if custom_transform.reversed {  // unable to go forward, haven't released key yet
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
            &mut TargetSpeed
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
        mut target_speed
    ) in datas.iter_mut()
    {
        let cursor_distance = cursor_pos.distance(transform.translation.xy());
        let max_speed = if custom_transform.reversed {
            - reverse_speed.0.get_raw()
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

        if speed_diff > acceleration.0.get_raw() {  // accelerating too much forwards
            custom_transform.speed.add_raw(acceleration.0.get_raw());
        } else if speed_diff < -acceleration.0.get_raw() {  // accelerating too much backwards
            custom_transform.speed.subtract_raw(acceleration.0.get_raw());
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
        let speed_diff = target_speed.0.get_raw() - custom_transform.speed.get_raw();
        if speed_diff > acceleration.0.get_raw() {
            custom_transform.speed.add_raw(acceleration.0.get_raw());
        } else if speed_diff < -acceleration.0.get_raw() {
            custom_transform
                .speed
                .subtract_raw(acceleration.0.get_raw());
        }
    }
}

/// updates [`Boat`]'s [`Transform`] according to its [`CustomTransform`]
fn update_transform(
    mut transform_ship: Query<
        (&mut Transform, &mut CustomTransform, &Children, &Sprite, &mut OutOfBound),
        With<Boat>,
    >,
    mut circle_huds: Query<&mut CircleHud>,
    world_size: Single<&WorldSize>,
) {
    for (mut transform, mut custom, children, sprite, mut out_of_bound) in transform_ship.iter_mut() {
        let Some(custom_size) = sprite.custom_size else {
            continue;
        };

        let mut translation = custom.position.to_vec3(transform.translation.z);
        
        translation += move_with_rotation(
            transform.rotation,
            custom.speed.get_raw()
        ); // ignores frame lagging temporary

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
    mut ships: Query<(&mut Transform, &DivingSpeed, &SubKind, &BoatOwner), With<Boat>>,
    buttons: Res<ButtonInput<Key>>,
) {
    let (mut transform, diving_speed, subkind, _) = ships
        .iter_mut()
        .find(|(.., owner)| matches!(owner, BoatOwner::Player))
        .expect("Player died?");

    if (buttons.just_pressed(Key::Character("r".into())) || buttons.just_pressed(Key::Character("R".into())))
        && *subkind == SubKind::Submarine
    {
        transform.decrease_with_limit(diving_speed.0, OCEAN_FLOOR);
    }
}

fn update_diving_overlay(
    ship_pos: Query<&CustomTransform, With<Boat>>,
    transforms: Query<&Transform, With<Boat>>,
    mut diving_overlay_material: ResMut<Assets<DivingOverlay>>,
    diving_overlay: Query<&MeshMaterial2d<DivingOverlay>>
) {
    // currently ignores possibility of multiple ships
    let Some(ship) = ship_pos.iter().last() else {
        return;
    };

    for id in diving_overlay {
        if let Some(diving_material) = diving_overlay_material.get_mut(id) {
            diving_material.player_pos = ship.position.0;
        }
    }

}