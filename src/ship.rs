//! currently, there are no differentiation between a Ship and a Submarine
//! 
//! be mindful of [`Ship::transform`] and [`Transform`] of the [`Ship`] needs to be kept in sync

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::util::{calculate_from_proportion, get_cursor_pos, add_one_third, get_rotate_radian, move_with_rotation};
use crate::constants::*;
use crate::util::MainCamera;

#[derive(Component, Debug, Clone)]
pub struct Ship {
    /// maximum angle in radians that you can turn per frame, consider deriving from `max_speed`
    /// ### Warning
    /// keep the value small
    max_turn_radian: f32,
    /// max speed that the Ship can have
    max_speed: f32,
    /// the scale of yasen drawn
    scale: f32,
    /// tranform to update in seperate system
    transform: Transform,
    /// raw sprite size
    raw_size: f32,
}

impl Ship {
    fn radius(&self) -> f32 {
        self.raw_size * self.scale / 2.0
    }
}

pub fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    let transform = Transform::from_scale(Vec3::splat(0.3))
        .with_translation(Vec3 { x: 100.0, y: 0.0, z: 0.0 })
        .with_rotation(Quat::from_rotation_z(PI / (180.0 / 90.0)));

    commands.spawn((Camera2d, MainCamera));  // ::default() ?
    commands.spawn((  // TODO Bundle it
        Sprite::from_image(
            asset_server.load("yasen.png")
        ),
        transform,
        Ship {
            max_turn_radian: YASEN_MAX_TURN_DEGREE.to_radians(),
            max_speed: YASEN_MAX_SPEED,
            scale: 0.3,
            transform,
            raw_size: YASEN_RAW_SIZE
        },
        Mesh2d(
            meshes.add(Circle::new(add_one_third(YASEN_RAW_SIZE / 2.0)))
        )
    ));
}

/// modifys [`Transform`] of [`Ship`]
pub fn update_ship(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut ship_data: Query<(&Transform, &mut Ship)>,  // decided to read Transform from Transform instead of Ship for actual value
) {
    if let Some(cursor_pos) = get_cursor_pos(window, camera) && buttons.pressed(MouseButton::Left) {
        rotate_ship(&mut ship_data, cursor_pos);
        move_ship(&mut ship_data, cursor_pos);
    }
}

/// handle rotation
fn rotate_ship(ship_data: &mut Query<(&Transform, &mut Ship)>, cursor_pos: Vec2) {
    for (transform, mut ship) in ship_data.iter_mut() {  // TODO consider subtracting this into the system

        let raw_moved = get_rotate_radian(cursor_pos, transform.translation.xy());
        let (_, _, current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

        let moved = {
            let mut raw_moved = (raw_moved.to_degrees() - current_rotation.to_degrees()).to_radians();
            if raw_moved > PI {
                raw_moved -= 2.0 * PI;
            } else if raw_moved < -PI {
                raw_moved += 2.0 * PI;
            }
            raw_moved
        };

        if moved.abs() > ship.max_turn_radian {
            let ship_max_turn = ship.max_turn_radian;
            if moved > 0.0 {
                ship.transform.rotate_local_z(ship_max_turn);
            } else if moved < 0.0 {
                ship.transform.rotate_local_z(-ship_max_turn);
            }
        } else {
            ship.transform.rotate_local_z(moved);
        }

        println!("Moved from last time: {}", moved.to_degrees());
    }
}

/// handle moving
fn move_ship(ship_data: &mut Query<(&Transform, &mut Ship)>, cursor_pos: Vec2) {
    for (transform, mut ship) in ship_data.iter_mut() {
        
        let cursor_distance = cursor_pos.distance(transform.translation.xy());
        let speed = calculate_from_proportion(
            cursor_distance,
            add_one_third(ship.radius()),
            ship.max_speed,
            ship.radius()
        );
        
        ship.transform.translation += move_with_rotation(transform.rotation, speed);
    }
}

/// updates [`Ship`]'s [`Transform`] along with Circle HUD
pub fn update_transform(mut transform_ship: Query<(&mut Transform, &Ship)>) {
    for (mut transform, ship) in transform_ship.iter_mut() {
        *transform = ship.transform;
    }
}
