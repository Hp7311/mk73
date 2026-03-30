use bevy::prelude::*;

use crate::{DEFAULT_MAX_TURN_DEG, primitives::Speed};

#[derive(Debug, Component, Clone, Copy)]
pub enum Weapon {
    Set65,
}

#[derive(Debug, Copy, Clone)]
enum WeaponType {
    Torpedo,
}

impl Weapon {
    fn file_name(&self) -> &'static str {
        match self {
            Weapon::Set65 => "Set65.png",
        }
    }
    fn custom_size(&self) -> Vec2 {
        match self {
            Weapon::Set65 => vec2(25.6, 2.0),
        }
    }
    fn weapon_type(&self) -> WeaponType {
        match self {
            Weapon::Set65 => WeaponType::Torpedo,
        }
    }
    fn max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Weapon::Set65 => 50.0,
        })
    }
    fn acceleration(&self) -> Speed {
        Speed::from_knots(match self {
            Weapon::Set65 => 10.0,
        })
    }
    fn max_turn_radian(&self) -> f32 {
        match self {
            Weapon::Set65 => DEFAULT_MAX_TURN_DEG * 3.0,
        }
        .to_radians()
    }
}
