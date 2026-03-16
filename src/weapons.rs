use bevy::prelude::*;

use crate::{DEFAULT_MAX_TURN_DEG, primitives::{Acceleration, DecimalPoint, Speed, TargetRotation, TrimRadian}, util::eq};

const MAX_TURN_RADIAN: f32 = DEFAULT_MAX_TURN_DEG.to_radians();

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnWeaponMessage>()
            .add_systems(Update, spawn_weapon)
            .add_systems(Update, move_weapon);
    }
}

#[derive(Debug, Component, Clone, Copy)]
pub(crate) enum Weapon {
    Set65  // TODO seperate torp/shell/etc
}

#[derive(Debug, Copy, Clone)]
enum WeaponType {
    Torpedo
}

impl Weapon {
    // TODO do similar thing in boat.rs instead of constants
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
    fn speed(&self) -> Speed {
        Speed::from_knots(match self {
            Weapon::Set65 => 30.0
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
    pub target_rotation: f32  // Z-rotation in radians
}

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
            TargetRotation(Some(*target_rotation)),  // guaranteed Some, stores quat
            Weapon::Set65
        ));
        println!("Spawned one")
    }
}

fn move_weapon(mut weapons: Query<(&mut Transform, &TargetRotation, &Weapon)>) {
    for (mut transform, target_rotation, weapon) in weapons.iter_mut() {
        let current_rotation = transform.rotation.to_euler(EulerRot::XYZ).2;

        let moved_from_current = (target_rotation.0.unwrap().to_degrees() - current_rotation.to_degrees())
            .to_radians()
            .trim();

        if eq(moved_from_current, 0.0, DecimalPoint::Three) {
            continue;
        }

        if moved_from_current.abs() > MAX_TURN_RADIAN {
            if moved_from_current < 0.0 {
                transform.rotate_local_z(-MAX_TURN_RADIAN);
            } else {
                transform.rotate_local_z(MAX_TURN_RADIAN);
            }
        } else {
            transform.rotate_local_z(moved_from_current);
        }

        // info!("{}", moved_from_current.to_degrees());
    }
}