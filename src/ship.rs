//! currently, there are no differentiation between a Ship and a Submarine
//!
//! be mindful of [`Ship::transform`] and [`Transform`] of the [`Ship`] needs to be kept in sync

use std::f32::consts::PI;

use bevy::camera_controller::pan_camera::PanCamera;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::constants::*;
use crate::primitives::*;
use crate::util::{
    MainCamera, add_circle_hud, calculate_from_proportion, get_cursor_pos, get_rotate_radian,
    move_with_rotation,
};


#[derive(Component, Debug, Clone)]
pub struct Ship;

pub fn startup(
    mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {

    commands.spawn((
        Camera2d,
        PanCamera {
            min_zoom: 1.0,
            max_zoom: DEFAULT_MAX_ZOOM,
            key_down: None,
            key_left: None,
            key_right: None,
            key_up: None,
            ..default()
        },
        MainCamera,
    ));

    commands.spawn((
        ShipBundle::new(
            YASEN_MAX_SPEED,
            vec2(100.0, 0.0),
            "yasen.png",
            asset_server.clone(),
            YASEN_RAW_SIZE / 2.0,
        ),
        Ship
    ));
}

/// modifys [`Transform`] of [`Ship`]
pub fn update_ship(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut queries: ParamSet<(
        Query<(&Transform, &mut CustomTransform, &Radian), With<Ship>>,
        Query<(&Transform, &mut CustomTransform, &Radius, &Speed), With<Ship>>
    )>
) {
    if let Some(cursor_pos) = get_cursor_pos(window, camera)
        && buttons.pressed(MouseButton::Left)
    {
        rotate_ship(&mut queries.p0(), cursor_pos);
        move_ship(&mut queries.p1(), cursor_pos);
    }
}

/// handle rotation
fn rotate_ship(
    transforms: &mut Query<(&Transform, &mut CustomTransform, &Radian), With<Ship>>,
    cursor_pos: Vec2
) {
    for (transform, mut custom_transform, max_turn) in transforms.iter_mut() {
        // TODO consider subtracting this into the system

        let raw_moved = get_rotate_radian(cursor_pos, transform.translation.xy());
        let (_, _, current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

        let moved = {
            let mut raw_moved =
                (raw_moved.to_degrees() - current_rotation.to_degrees()).to_radians();
            if raw_moved > PI {
                raw_moved -= 2.0 * PI;
            } else if raw_moved < -PI {
                raw_moved += 2.0 * PI;
            }
            raw_moved
        };

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
    }
}

/// handle moving
fn move_ship(
    datas: &mut Query<(&Transform, &mut CustomTransform, &Radius, &Speed), With<Ship>>,
    cursor_pos: Vec2
) {
    for (transform, mut custom_transform, radius, max_speed) in datas.iter_mut() {
        let cursor_distance = cursor_pos.distance(transform.translation.xy());
        let speed = calculate_from_proportion(
            cursor_distance,
            add_circle_hud(radius.default_convert().0),
            max_speed.0,
            radius.default_convert().0,
        );

        println!("Speed: {}", speed);

        custom_transform.speed = Speed(speed);  // TODO currently not using Speed in custom
    }
}

/// updates [`Ship`]'s [`Transform`] along with Circle HUD
pub fn update_transform(mut transform_ship: Query<(&mut Transform, &mut CustomTransform), With<Ship>>) {
    for (mut transform, mut custom) in transform_ship.iter_mut() {
        let mut translation = custom.position.to_vec3();
        translation += move_with_rotation(transform.rotation, custom.speed.0);

        let target = Transform {
            translation,
            rotation: custom.rotation.to_quat(),
            scale: Vec3::splat(1.0)
        };
        *transform = target;

        custom.position = Position(translation.xy());  // sync position
    }
}

/// resize the Sprite
pub fn set_sprite_scale(mut sprites: Query<&mut Sprite>, assets: Res<Assets<Image>>) {
    for mut sprite in sprites.iter_mut() {
        let Some(image) = assets.get(&mut sprite.image) else { continue };
        if sprite.custom_size.is_some() { continue }

        println!("Changing size..");
        sprite.custom_size = Some(vec2(
            image.width() as f32 * DEFAULT_SPRITE_SHRINK,
            image.height() as f32 * DEFAULT_SPRITE_SHRINK,
        ));
    }
}