use bevy::prelude::*;

use crate::{DEFAULT_MAX_TURN_DEG, primitives::{Acceleration, DecimalPoint, MousePos, Speed, TrimRadian}, util::{eq, get_rotate_radian, move_with_rotation, vec2_eq}};

const MAX_TURN_RADIAN: f32 = DEFAULT_MAX_TURN_DEG.to_radians();

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnWeaponMessage>()
            .add_systems(Update, spawn_weapon)
            .add_systems(Update, (rotate_weapon, move_weapon, check_reached).chain());
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
            Weapon::Set65 => 40.0
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
    pub mouse_pos: Vec2
}

// TODO direct the torpedo toward the mouse pos, not in a direction
fn spawn_weapon(mut commands: Commands, mut reader: MessageReader<SpawnWeaponMessage>, asset_server: Res<AssetServer>) {
    for SpawnWeaponMessage { weapon, position, rotation, mouse_pos } in reader.read() {
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
            MousePos(Some(*mouse_pos)),
            Speed::from_knots(0.0),
            Weapon::Set65
        ));
        println!("Spawned one")
    }
}

fn rotate_weapon(mut query: Query<(&mut Transform, &MousePos), With<Weapon>>) {
    for (mut transform, mouse_pos) in query.iter_mut() {
        let Some(mouse_pos) = mouse_pos.0 else { continue; };  // TODO don't continue if guided
        let target_rotation = get_rotate_radian(mouse_pos, transform.translation.xy());
        let current_rotation = transform.rotation.to_euler(EulerRot::XYZ).2;

        let moved_from_current = (target_rotation.to_degrees() - current_rotation.to_degrees())
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

/// we currently don't want a Weapon to go in circles toward the firing target
fn check_reached(mut query: Query<(&Transform, &mut MousePos), With<Weapon>>) {
    for (transform, mut mouse_pos) in query.iter_mut() {
        let Some(pos) = mouse_pos.0 else { continue };
        if vec2_eq(transform.translation.xy(), pos, DecimalPoint::TwentyPixels) {  // FIXME torpedo goes in circles at certain levels
            mouse_pos.0 = None;
        }
    }
}