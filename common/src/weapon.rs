use bevy::prelude::*;
use macros::{FetchSprite, MaxSpeed, Reload, Size, WeaponType};
use serde::{Deserialize, Serialize};
use crate::primitives::Radian;
use crate::{DEFAULT_MAX_TURN_DEG, primitives::Speed};

#[derive(FetchSprite, Size, WeaponType, MaxSpeed, Reload, Debug, Component, Clone, Copy, PartialEq, Serialize, Deserialize, Hash, Eq)]
#[allow(non_camel_case_types)]
pub enum Weapon {
    #[length = 6.2]
    #[weapon_type = "Torpedo"]
    #[max_speed = 29]
    #[reload = 8]
    Mark18,
    // Shell_heightxlengthMmr  length and height in milimeters (apparently since mk48 divide it by 1000)
    #[max_speed = 1243.6]
    #[reload = 8.7]
    Shell_57x441Mmr,
    #[length = 0.4]
    #[weapon_type = "DepthCharge"]
    #[max_speed = "None"]  // TODO what?
    #[reload = 16]
    Mark9,
    #[length = 7.2]
    #[weapon_type = "Torpedo"]
    #[max_speed = 45.1]
    #[reload = 8]
    Type53,
    #[max_speed = 1312.1]
    #[reload = 8]
    Shell_25x129Mmr,
    #[max_speed = 1151.7]
    #[reload = 9.7]
    Shell_127x680Mmr,
    #[length = 1.1]
    #[weapon_type = "Rocket"]
    #[max_speed = 388.8]
    #[reload = 2.5]
    Of45,
    #[max_speed = 1332.5]
    #[reload = 12.6]
    Shell_300x1400Mmr,
    #[length = 7.9]
    #[weapon_type = "Torpedo"]
    #[max_speed = 40]
    #[reload = 12]
    Set65,
    #[length = 8.4]
    #[weapon_type = "Missle"]
    #[max_speed = 1931.9]
    #[reload = 12]
    BrahMos,
    /// following name on https://mk48.io/ships/yasen/
    #[length = 6.5]
    #[weapon_type = "RocketTorpedo"]
    #[max_speed = 388.8]
    #[reload = 20]
    #[json = "Rpk6"]
    Vodopad,  // todo associated 82R
    #[length = 1.6]
    #[weapon_type = "AntiAir"]
    #[max_speed = 1108]
    #[reload = 16]
    Igla,
    #[length = 1.5]
    #[weapon_type = "SonarDecoy"]
    #[max_speed = 23.2]
    #[reload = 20]
    Brosok
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum WeaponType {
    Torpedo,
    Shell,
    Missle,
    Rocket,
    RocketTorpedo,
    DepthCharge,
    AntiAir,
    SonarDecoy
}

impl Weapon {
    pub fn acceleration(&self) -> Speed {
        if matches!(self.weapon_type(), WeaponType::Shell | WeaponType::Rocket) {
            return Speed::from_raw(f32::MAX)  // from raw to avoid overflow
        }
        Speed::from_knots(match self {
            Weapon::Set65 => 10.0,
            _ => 10.0 // for now
        })
    }
    /// turn speed
    pub fn max_turn_radian(&self) -> Radian {
        if matches!(self.weapon_type(), WeaponType::Shell | WeaponType::Rocket) {
            return Radian(f32::MAX)
        }
        DEFAULT_MAX_TURN_DEG * match self {
            Self::Set65 => 3.0,
            _ => 3.0 // for now
        }
    }
}

impl WeaponType {
    // todo make a marker component on static weapons for perf
    /// we're not moving static weapons for now
    pub fn is_static(&self) -> bool {
        matches!(self, Self::DepthCharge)
    }
}