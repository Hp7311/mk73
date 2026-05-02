use bevy::prelude::*;
use std::ops::Mul;
use serde::{Deserialize, Serialize};
use crate::primitives::Radian;
use crate::{DEFAULT_MAX_TURN_DEG, primitives::Speed};


#[derive(Debug, Component, Clone, Copy, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub enum Weapon {
    Set65
}

#[derive(Debug, Copy, Clone)]
enum WeaponType {
    Torpedo,
}

impl Weapon {
    pub fn file_name(&self) -> &'static str {
        match self {
            Weapon::Set65 => "Set65.png",
        }
    }
    pub fn custom_size(&self) -> Vec2 {
        match self {
            Weapon::Set65 => vec2(25.6, 2.0),
        }
    }
    pub fn weapon_type(&self) -> WeaponType {
        match self {
            Weapon::Set65 => WeaponType::Torpedo,
        }
    }
    pub fn max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Weapon::Set65 => 50.0,
        })
    }
    pub fn acceleration(&self) -> Speed {
        Speed::from_knots(match self {
            Weapon::Set65 => 10.0,
        })
    }
    pub fn max_turn_radian(&self) -> Radian {
        match self {
            Weapon::Set65 => DEFAULT_MAX_TURN_DEG * 3.0,
        }
    }
}
