use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    DEFAULT_MAX_TURN_DEG, DEFAULT_SPRITE_SHRINK,
    primitives::{CircleHud, CustomTransform, OutOfBound, Speed},
    protocol::{Reversed, Rotate},
    weapon::Weapon,
    world::WorldSize,
};
use crate::primitives::Radian;

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boat {
    Yasen,
}

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubKind {
    Submarine,
    SurfaceShip,
}

impl Boat {
    pub fn sub_kind(&self) -> SubKind {
        match self {
            Self::Yasen => SubKind::Submarine,
        }
    }
    pub fn file_name(&self) -> &'static str {
        match self {
            Self::Yasen => "yasen.png",
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
    pub fn max_turn(&self) -> Radian {
        DEFAULT_MAX_TURN_DEG
    }
    /// radius
    pub fn radius(&self) -> f32 {
        self.sprite_size().x / 2.0
    }
}
