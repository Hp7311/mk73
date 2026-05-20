use std::collections::HashMap;
use bevy::prelude::*;
use lightyear::core::id::PeerId;
use serde::{Deserialize, Serialize};

use crate::primitives::{FileName, Level, Radian};
use crate::{
    DEFAULT_MAX_TURN_DEG, DEFAULT_SPRITE_SHRINK,
    primitives::Speed,
    weapon::Weapon,
};

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boat {
    Zubr,
    Momi,
    Yasen,
}

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubKind {
    Submarine,
    SurfaceShip,
    HoverCraft,
}

impl Boat {
    pub const ALL: [Self; 3] = [Self::Yasen, Self::Momi, Self::Zubr];
    pub fn sub_kind(&self) -> SubKind {
        match self {
            Self::Zubr => SubKind::HoverCraft,
            Self::Momi => SubKind::SurfaceShip,
            Self::Yasen => SubKind::Submarine,
        }
    }
    pub fn level(&self) -> Level {
        match self {
            Self::Zubr => Level::Two,
            Self::Momi => Level::Two,
            Self::Yasen => Level::Eight
        }
    }
    pub fn file_name(&self) -> FileName {
        FileName(match self {
            Self::Zubr => "zubr.png",
            Self::Momi => "momi.png",
            Self::Yasen => "yasen.png",
        })
    }
    pub fn armanents(&self) -> HashMap<Weapon, u8> {
        HashMap::from(match self {
            // TODO
            Self::Zubr => [(Weapon::Set65, 1),],
            Self::Momi => [(Weapon::Set65, 1),],

            Self::Yasen => [(Weapon::Set65, 4),]
        })
    }
    pub fn default_weapon(&self) -> Option<Weapon> {
        match self {
            // TODO
            Self::Zubr => None,
            Self::Momi => None,

            Self::Yasen => Some(Weapon::Set65),
        }
    }
    pub fn max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Zubr => 55.0,
            Self::Momi => 36.0,
            Self::Yasen => 35.0,
        })
    }
    pub fn rev_max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Zubr => 29.0,
            Self::Momi => 31.0,
            Self::Yasen => 21.0,
        })
    }
    pub fn diving_speed(&self) -> Speed {
        Speed::from_raw(match self {
            Self::Zubr => 0.004,  // FIXME
            Self::Momi => 0.005,
            Self::Yasen => 0.004,
        })
    }
    pub fn acceleration(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Zubr => 1.0,
            Self::Momi => 1.0,
            Self::Yasen => 1.0,
        })
    }
    /// vec2(width, height)
    pub fn sprite_size(&self) -> Vec2 {
        (match self {
            Self::Momi => vec2(1024.0, 91.0),
            Self::Zubr => vec2(512.0, 190.0),
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
    /// should use this function or code will break
    pub fn circle_hud_radius(&self) -> f32 {
        crate::util::add_circle_hud(self.radius())
    }
}


/// identifying the perticular [`Boat`]
#[derive(Debug, Component)]
pub struct BoatClientId(pub PeerId);
