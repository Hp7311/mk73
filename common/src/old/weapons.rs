use std::{f32::consts::PI, ops::Range};

use bevy::{color::palettes::css::LIME, prelude::*};
use rand::RngExt;

use crate::{
    CIRCLE_HUD, DEFAULT_MAX_TURN_DEG,
    primitives::{DecimalPoint, MeshBundle, NormalizeRadian, Speed, TargetRotation},
    util::{eq, move_with_rotation},
};

/// client
pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnWeaponMessage>()
            .add_systems(Update, spawn_weapon)
            .add_systems(
                Update,
                (rotate_weapon, move_weapon, sync_weapon_marker).chain(),
            );
    }
}

#[derive(Debug, Component, Clone, Copy)]
pub(crate) enum Weapon {
    Set65,
}

/// the green/red marker above a [`Weapon`]
#[derive(Debug, Component, Clone, Copy)]
struct WeaponMarker(Entity);

/// inter-mod message to spawn a Weapon
#[derive(Debug, Message)]
pub(crate) struct SpawnWeaponMessage {
    pub weapon: Weapon,
    pub position: Vec2,
    pub rotation: Quat,
    pub target_rotation: Quat,
}

fn spawn_weapon(
    mut commands: Commands,
    mut reader: MessageReader<SpawnWeaponMessage>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for SpawnWeaponMessage {
        weapon,
        position,
        rotation,
        target_rotation,
    } in reader.read()
    {
        let weapon_id = commands
            .spawn((
                Sprite {
                    image: asset_server.load(weapon.file_name()),
                    custom_size: Some(weapon.custom_size()),
                    ..default()
                },
                Transform {
                    translation: position.extend(0.0),
                    rotation: *rotation,
                    ..default()
                },
                TargetRotation(Some(target_rotation.to_euler(EulerRot::XYZ).2)), // cannot be None
                Speed::from_knots(0.0),
                Weapon::Set65,
            ))
            .id();

        commands.spawn((
            MeshBundle {
                mesh: Mesh2d(meshes.add(Triangle2d::new(
                    vec2(-10.0, 0.0),
                    vec2(0.0, 15.0),
                    vec2(10.0, 0.0),
                ))),
                materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(LIME))),
            },
            Transform {
                translation: position.extend(CIRCLE_HUD), // higher Z-ordering
                rotation: Quat::from_rotation_z(WeaponMarker::TRIANGLE_ROTATION),
                ..default()
            },
            WeaponMarker(weapon_id),
        ));
    }
}

fn rotate_weapon(mut query: Query<(&mut Transform, &TargetRotation, &Weapon)>) {
    for (mut transform, target_rotation, weapon) in query.iter_mut() {
        let max_turn_radian = weapon.max_turn_radian();
        let current_rotation = transform.rotation.to_euler(EulerRot::XYZ).2;

        let moved_from_current = (target_rotation.unwrap() - current_rotation).normalize();

        if eq(moved_from_current, 0.0, DecimalPoint::Three) {
            continue;
        }

        if moved_from_current.abs() > max_turn_radian {
            if moved_from_current < 0.0 {
                transform.rotate_local_z(-max_turn_radian);
            } else {
                transform.rotate_local_z(max_turn_radian);
            }
        } else {
            transform.rotate_local_z(moved_from_current);
        }
    }
}

// currently calculating a rotation once and passed here to spawn a weapon.

fn move_weapon(mut query: Query<(&mut Transform, &Weapon, &mut Speed)>) {
    for (mut transform, weapon, mut last_speed) in query.iter_mut() {
        let mut speed = *last_speed.as_ref();
        let speed_diff = weapon.max_speed().get_raw() - last_speed.get_raw();
        let acceleration = weapon.acceleration().get_raw();

        if speed_diff > acceleration {
            speed.add_raw(acceleration);
        } else if speed_diff < -acceleration {
            speed.subtract_raw(acceleration);
        } else if speed_diff.abs() > 0.1 {
            speed.overwrite_with_raw(weapon.max_speed().get_raw());
        } // weapons don't have dynamic speeds. therefore it'll always try to go at max speed

        *last_speed = speed;

        // update transform
        let move_by = move_with_rotation(transform.rotation, speed.get_raw());
        transform.translation += move_by;
    }
}

fn sync_weapon_marker(
    weapons: Query<&Transform, With<Weapon>>,
    mut markers: Query<(&mut Transform, &WeaponMarker), Without<Weapon>>,
) {
    for (mut transform, marker) in markers.iter_mut() {
        if let Ok(parent_transform) = weapons.get(marker.0) {
            transform.translation.x = parent_transform.translation.x;
            transform.translation.y = parent_transform.translation.y + WeaponMarker::Y_OFFSET;
        } else {
            // TODO despawn?
        }
    }
}

impl WeaponMarker {
    /// how much to rotate the [`Mesh2d`] triangle
    const TRIANGLE_ROTATION: f32 = PI;
    /// when attaching to weapon
    const Y_OFFSET: f32 = 30.0;
}
