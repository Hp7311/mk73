use bevy::prelude::*;
use macros::{FetchSprite, MaxSpeed, Size, WeaponType};
use serde::{Deserialize, Serialize};
use crate::primitives::{FetchSprite, Radian};
use crate::{DEFAULT_MAX_TURN_DEG, primitives::Speed};

// TODO macro for this too

#[derive(FetchSprite, Size, WeaponType, MaxSpeed, Debug, Component, Clone, Copy, PartialEq, Serialize, Deserialize, Hash, Eq)]
#[allow(non_camel_case_types)]
pub enum Weapon {
    #[length = 6.2]
    #[weapon_type = "Torpedo"]
    #[max_speed = 29]
    Mark18,
    // Shell_lengthxheightMmr  length and height in milimeters (apparently since mk48 divide it by 1000)
    #[max_speed = 1243.6]
    Shell_57x441Mmr,
    #[length = 0.4]
    #[weapon_type = "DepthCharge"]
    #[max_speed = "None"]
    Mark9,
    #[length = 7.2]
    #[weapon_type = "Torpedo"]
    #[max_speed = 45.1]
    Type53,
    #[max_speed = 1312.1]
    Shell_25x129Mmr,
    #[max_speed = 1151.7]
    Shell_127x680Mmr,
    #[length = 1.1]
    #[weapon_type = "Rocket"]
    #[max_speed = 1312.1]
    Of45,
    #[length = 7.9]
    #[weapon_type = "Torpedo"]
    #[max_speed = 40]
    Set65,
    #[length = 8.4]
    #[weapon_type = "Missle"]
    #[max_speed = 1931.9]
    BrahMos,
    /// following name on https://mk48.io/ships/yasen/
    #[length = 6.5]
    #[weapon_type = "RocketTorpedo"]
    #[max_speed = 388.8]
    #[json = "Rpk6"]
    Vodopad,  // TODO associated 82R
    #[length = 1.6]
    #[weapon_type = "AntiAir"]
    #[max_speed = 1108]
    Igla,
    #[length = 1.5]
    #[weapon_type = "SonarDecoy"]
    #[max_speed = 23.2]
    Brosok
}

#[derive(Debug, Copy, Clone)]
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
        Speed::from_knots(match self {
            Weapon::Set65 => 10.0,
            _ => 10.0 // for now
        })
    }
    /// turn speed
    pub fn max_turn_radian(&self) -> Radian {
        DEFAULT_MAX_TURN_DEG * match self {
            Self::Set65 => 3.0,
            _ => 3.0 // for now
        }
    }
}

impl WeaponType {
    // TODO make a marker component on static weapons for perf
    /// we're not moving static weapons for now
    pub fn is_static(&self) -> bool {
        matches!(self, Self::DepthCharge)
    }
}