use std::ops::Range;

use bevy::prelude::*;
use rand::RngExt;

use crate::{DEFAULT_MAX_TURN_DEG, primitives::{Acceleration, DecimalPoint, Speed, TargetRotation, TrimRadian}, util::{eq, move_with_rotation}};

/// faster max turning speed for torpedoes
const MAX_TURN_RADIAN: Range<f32> = (DEFAULT_MAX_TURN_DEG * 2.0 ).to_radians()..(DEFAULT_MAX_TURN_DEG * 3.0).to_radians();

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnWeaponMessage>()
            .add_systems(Update, spawn_weapon)
            .add_systems(Update, (rotate_weapon, move_weapon).chain());
    }
}

#[derive(Debug, Component, Clone, Copy)]
pub(crate) enum Weapon {
    Set65
}

#[derive(Debug, Copy, Clone)]
enum WeaponType {
    Torpedo
}

impl Weapon {
    fn file_name(&self) -> &'static str {
        match self {
            Weapon::Set65 => "Set65.png"
        }
    }
    fn custom_size(&self) -> Vec2 {
        match self {
            Weapon::Set65 => vec2(25.6, 2.0)
        }
    }
    // TODO macro for these matching enums
    fn weapon_type(&self) -> WeaponType {
        match self {
            Weapon::Set65 => WeaponType::Torpedo
        }
    }
    fn max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Weapon::Set65 => 50.0
        })
    }
    fn acceleration(&self) -> Acceleration {
        Acceleration(Speed::from_knots(match self {
            Weapon::Set65 => 10.0
        }))
    }
}

#[derive(Debug, Message)]
pub(crate) struct SpawnWeaponMessage{
    pub weapon: Weapon,
    pub position: Vec2,
    pub rotation: Quat,
    pub target_rotation: Quat
}

// TODO direct the torpedo toward the mouse pos, not in a direction
fn spawn_weapon(mut commands: Commands, mut reader: MessageReader<SpawnWeaponMessage>, asset_server: Res<AssetServer>) {
    for SpawnWeaponMessage { weapon, position, rotation, target_rotation } in reader.read() {
        commands.spawn((
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
            TargetRotation(Some(target_rotation.to_euler(EulerRot::XYZ).2)),  // cannot be None
            Speed::from_knots(0.0),
            Weapon::Set65
        ));
    }
}

fn rotate_weapon(mut query: Query<(&mut Transform, &TargetRotation), With<Weapon>>) {
    for (mut transform, target_rotation) in query.iter_mut() {
        let max_turn_radian = rand::rng().random_range(MAX_TURN_RADIAN);
        let current_rotation = transform.rotation.to_euler(EulerRot::XYZ).2;

        let moved_from_current = (target_rotation.unwrap().to_degrees() - current_rotation.to_degrees())
            .to_radians()
            .trim();

        if eq(moved_from_current, 0.0, DecimalPoint::Three) {
            // dbg!(moved_from_current);
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

        // info!("{}", moved_from_current.to_degrees());
    }
}

// currently calculating a rotation once and passed here to spawn a weapon.

fn move_weapon(mut query: Query<(&mut Transform, &Weapon, &mut Speed)>) {
    for (mut transform, weapon, mut last_speed) in query.iter_mut() {
        let mut speed = last_speed.as_ref().clone();
        let speed_diff = weapon.max_speed().get_raw() - last_speed.get_raw();
        let acceleration = weapon.acceleration().get_raw();

        if speed_diff > acceleration {
            speed.add_raw(acceleration);
        } else if speed_diff < -acceleration {
            speed.subtract_raw(acceleration);
        }

        else if speed_diff.abs() > 0.1 {
            speed.overwrite_with_raw(weapon.max_speed().get_raw());
        }  // weapons don't have dynamic speeds. therefore it'll always try to go at max speed

        *last_speed = speed;

        // update transform

        let move_by = move_with_rotation(transform.rotation, speed.get_raw());
        transform.translation += move_by;
    }
}
