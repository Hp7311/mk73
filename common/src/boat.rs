use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{DEFAULT_MAX_TURN_DEG, DEFAULT_SPRITE_SHRINK, primitives::Speed, weapon::Weapon};

/// pub to send data between client and server
#[derive(Component, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Boat {
    pub data: BoatData,
    pub subkind: SubKind, // should be only one `Boat` in client so no owner
}

#[derive(Component, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum BoatData {
    Yasen,
}

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubKind {
    Submarine,
    SurfaceShip,
}

impl BoatData {
    pub fn file_name(&self) -> &'static str {
        match self {
            Self::Yasen => "yasen.png"
        }
    }
    pub fn get_armanents(&self) -> Vec<Weapon> {
        match self {
            Self::Yasen => vec![Weapon::Set65],
        }
    }
    pub fn default_weapon(&self) -> Option<Weapon> {
        match self {
            Self::Yasen => Some(Weapon::Set65),
        }
    }
    pub fn max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Yasen => 35.0,
        })
    }
    pub fn rev_max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Yasen => 21.0,
        })
    }
    pub fn diving_speed(&self) -> Speed {
        Speed::from_raw(match self {
            Self::Yasen => 0.004,
        })
    }
    pub fn acceleration(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Yasen => 2.0,
        })
    }
    /// raw file size * [`DEFAULT_SPRITE_SHRINK`]
    pub fn sprite_size(&self) -> Vec2 {
        (match self {
            Self::Yasen => vec2(1024.0, 156.0),
        }) * DEFAULT_SPRITE_SHRINK
    }
    /// max turn in degrees
    pub fn max_turn(&self) -> f32 {
        DEFAULT_MAX_TURN_DEG
    }
}
