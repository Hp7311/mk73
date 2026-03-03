//! currently, there are no differentiation between a Ship and a Submarine
//!
//! be mindful of [`Ship::transform`] and [`Transform`] of the [`Ship`] needs to be kept in sync

// doc outdated

use std::f32::consts::PI;

use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::MainCamera;
use crate::DEFAULT_SPRITE_SHRINK;
use crate::primitives::*;
use crate::util::fill_dimensions;
use crate::collision::out_of_bounds;
use crate::util::resize_inner;
use crate::util::{
    add_circle_hud, calculate_from_proportion, get_cursor_pos, get_rotate_radian,
    move_with_rotation,
};
use crate::world::WorldSize;

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup)
            .add_systems(Update, (
                resize_ship,
                update_ship,
                update_transform
            ).chain())
            .add_systems(PostUpdate, move_camera.after(TransformSystems::Propagate));
    }
}


#[derive(Component, Debug, Copy, Clone)]
struct Ship;

const YASEN_MAX_SPEED: f32 = 1.5;  // using HashMap?
const YASEN_BACK_SPEED: f32 = 0.9;
const YASEN_ACCELERATION: f32 = 0.03;

const YASEN_RAW_SIZE: f32 = 1024.0;
/// absolute value of minimum radians that must be reached to reverse the Ship
const MINIMUM_REVERSE: f32 = PI * (2.0 / 3.0);


// TODO constants for Z-ordering
fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {

    let radius = add_circle_hud(YASEN_RAW_SIZE / 2.0) * DEFAULT_SPRITE_SHRINK;
    commands.spawn((
        ShipBundle::new(
            YASEN_MAX_SPEED,
            YASEN_BACK_SPEED,
            YASEN_ACCELERATION,
            vec2(100.0, 0.0),
            "yasen.png",
            asset_server.clone(),
            YASEN_RAW_SIZE / 2.0,
        ),
        Ship,
    ))
    .with_children(|parent | {
        parent.spawn((
            CircleHudBundle {
                mesh: Mesh2d(meshes.add(Circle::new(radius)
                        .to_ring(3.0),
                )),
                materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
            },
            Transform::from_translation(vec3(0.0, 0.0, 30.0)),  // relative to parent, circle hud highest Z
            CircleHud { radius, center: vec2(100.0, 0.0) }
        ));
    });
}


/// helper struct for accessing the [`Ship`](crate::ship::Ship)'s circle HUD
#[derive(Debug, Component, Copy, Clone)]
pub(crate) struct CircleHud {
    pub(crate) radius: f32,
    pub(crate) center: Vec2
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
            DecimalPoint::Three => 0.001
        };

        x_diff < max_distance && y_diff < max_distance
    }
}

fn move_camera(
    mut camera: Single<&mut Transform, With<MainCamera>>,
    ship_pos: Query<&CustomTransform, With<Ship>>,
) {
    // currently ignores possibility of multiple ships
    let Some(ship) = ship_pos.iter().last() else {
        return;
    };

    if ship.position.0 != camera.translation.xy() {
        camera.translation = ship.position.0.extend(0.0);
    }
}

/// modifys [`Transform`] of [`Ship`]
fn update_ship(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut queries: ParamSet<(
        Query<(&Transform, &mut CustomTransform, &Radian, &mut TargetRotation, &mut ReleasedAfterReverse), With<Ship>>,
        Query<(&Transform, &mut CustomTransform, &Radian, &TargetRotation, &TargetSpeed, &Acceleration), With<Ship>>,
        Query<(&Transform, &mut CustomTransform, &Radius, &MaxSpeed, &ReverseSpeed, &Acceleration, &mut TargetSpeed), With<Ship>>,
        Query<(&CustomTransform, &mut ReleasedAfterReverse), With<Ship>>
    )>
) {
    if let Some(cursor_pos) = get_cursor_pos(&window, &camera)
        && buttons.pressed(MouseButton::Left)
    {
        rotate_ship(&mut queries.p0(), cursor_pos);
        move_ship(&mut queries.p2(), cursor_pos);
    } else {
        ship_to_target(&mut queries.p1());
    }
    if get_cursor_pos(&window, &camera).is_some()
        && buttons.just_released(MouseButton::Left)
    {
        try_release_after_rev(&mut queries.p3())
    }
}

/// handle rotation
fn rotate_ship(
    transforms: &mut Query<(&Transform, &mut CustomTransform, &Radian, &mut TargetRotation, &mut ReleasedAfterReverse), With<Ship>>,
    cursor_pos: Vec2,
) {
    for (transform, mut custom_transform, max_turn, mut target_rotation, mut released_after_reverse) in transforms.iter_mut() {

        let raw_moved = get_rotate_radian(cursor_pos, transform.translation.xy());  // diff from radian 0
        let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);
        let mut target_move = raw_moved;

        let moved = {  // radians to move from current rotation
            let mut moved_from_current = (raw_moved.to_degrees() - current_rotation.to_degrees())
                .to_radians()
                .trim();

            // if reversing, adjust return value
            if moved_from_current.abs() > MINIMUM_REVERSE {
                custom_transform.reversed = true;
                moved_from_current = moved_from_current.flip();
                target_move = target_move.flip()

            } else if custom_transform.reversed && released_after_reverse.0 {  // free to forward again
                custom_transform.reversed = false;
                released_after_reverse.0 = false;  // reset. setting to true is done in `try_release_after_rev`
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
        } else { // normal
            custom_transform.rotate_local_z(moved.to_radian_unchecked());
        }

        
        *target_rotation = Some(target_move).into();
    }
}

fn try_release_after_rev(query: &mut Query<(&CustomTransform, &mut ReleasedAfterReverse), With<Ship>>) {
    for (CustomTransform { reversed, ..}, mut release) in query {
        if !reversed { continue; }

        release.0 = true;
    }
}

/// handle moving
fn move_ship(
    datas: &mut Query<(&Transform, &mut CustomTransform, &Radius, &MaxSpeed, &ReverseSpeed, &Acceleration, &mut TargetSpeed), With<Ship>>,
    cursor_pos: Vec2,
) {
    for (transform, mut custom_transform, radius, max_speed, reverse_speed, acceleration, mut target_speed) in datas.iter_mut() {
        let cursor_distance = cursor_pos.distance(transform.translation.xy());
        let speed = if custom_transform.reversed {
            reverse_speed.0
        } else {
            max_speed.0
        };

        let mut speed = calculate_from_proportion(
            cursor_distance,
            add_circle_hud(radius.default_convert().0),
            speed,
            radius.default_convert().0,
        );

        target_speed.0 = speed;

        // adjust for acceleration
        let speed_diff = speed - custom_transform.speed.0;
        if speed_diff > acceleration.0 {
            speed = custom_transform.speed.0 + acceleration.0;
        } else if speed_diff < -acceleration.0 {
            speed = custom_transform.speed.0 - acceleration.0;
        }

        custom_transform.speed = Speed(speed);
    }
}

// note that we're accepting Query instead of Single for ship everywhere
// and not descriminating Bot/Player

/// remember the last move angle and rotate toward it when button not pressed
fn ship_to_target(ships: &mut Query<(&Transform, &mut CustomTransform, &Radian, &TargetRotation, &TargetSpeed, &Acceleration), With<Ship>>) {
    for (transform, mut custom_transform, max_turn, target_rotation, target_speed, acceleration) in ships {
        // ------ rotation
        let Some(target_rotation) = target_rotation.0 else { continue; };

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
        let speed_diff = target_speed.0 - custom_transform.speed.0;
        if speed_diff > acceleration.0 {
            custom_transform.speed.0 = custom_transform.speed.0 + acceleration.0;
        } else if speed_diff < -acceleration.0 {
            custom_transform.speed.0 = custom_transform.speed.0 - acceleration.0;
        }
    }
}

/// updates [`Ship`]'s [`Transform`] according to its [`CustomTransform`]
 fn update_transform(
    mut transform_ship: Query<(&mut Transform, &mut CustomTransform, &Children, &Dimensions), With<Ship>>,
    mut circle_huds: Query<&mut CircleHud>,
    world_size: Single<&WorldSize>,
) {
    for (mut transform, mut custom, children, dimension) in transform_ship.iter_mut().filter(|(.., dimension)| dimension.0.is_some()) {
        let mut translation = custom.position.to_vec3();
        if custom.reversed {
            translation += move_with_rotation(transform.rotation, -custom.speed.0);
        } else {
            translation += move_with_rotation(transform.rotation, custom.speed.0);  // ignores frame lagging temporary
        }

        
        if out_of_bounds(&world_size, dimension.0.unwrap(), translation.xy(), custom.rotation.to_quat()) {
            println!("Out of bounds");
            return;
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
    }
}

fn resize_ship(
    mut queries: ParamSet<(
        Query<&mut Sprite, With<Ship>>,
        Query<(&Sprite, &mut Dimensions), With<Ship>>
    )>,
    assets: Res<Assets<Image>>
) {
    resize_inner(queries.p0(), &assets);
    fill_dimensions(queries.p1(), &assets);
}
