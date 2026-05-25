use bevy::prelude::*;
use lightyear::core::id::PeerId;
use serde::{Deserialize, Serialize};

use crate::primitives::{Level, Radian};
use crate::{
    DEFAULT_MAX_TURN_DEG, DEFAULT_SPRITE_SHRINK,
    primitives::Speed,
    weapon::Weapon,
};
use macros::BoatImpl;


/// for performance improvements in diving
#[derive(Resource, PartialEq)]
#[cfg(feature = "client")]
pub struct BoatType(pub SubKind);


#[derive(BoatImpl, Component, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boat {
    #[armanents(Set65)]
    #[level = 1]
    FiarmileD,
    #[armanents(Set65)]
    #[level = 1]
    G5,
    #[armanents(Set65)]
    #[level = 1]
    Komar,
    #[armanents(None)]
    #[level = 1]
    Olympias,
    #[armanents(Set65)]
    #[level = 1]
    Pt34,
    #[armanents(Set65)]
    #[level = 2]
    Zubr,
    #[armanents(Set65)]
    #[level = 2]
    Momi,
    #[armanents(Set65)]
    #[level = 8]
    Yasen,
}

#[derive(Debug, Component)]
pub struct CircleHud;

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubKind {
    Submarine,
    SurfaceShip,
    HoverCraft,
}

impl Boat {
    /// absolute value of minimum radians that must be reached to reverse the Boat
    pub const MINIMUM_REVERSE: Radian = Radian::from_deg(180.0 - 45.0);
    pub const ALL: [Self; 3] = [Self::Yasen, Self::Momi, Self::Zubr];
    pub fn sub_kind(&self) -> SubKind {
        match self {
            Self::Zubr => SubKind::HoverCraft,
            Self::Momi => SubKind::SurfaceShip,
            Self::Yasen => SubKind::Submarine,
            _ => todo!()
        }
    }
    pub fn default_weapon(&self) -> Option<Weapon> {
        match self {
            // TODO
            Self::Zubr => None,
            Self::Momi => None,

            Self::Yasen => Some(Weapon::Set65),
            _ => todo!()
        }
    }
    pub fn max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Zubr => 55.0,
            Self::Momi => 36.0,
            Self::Yasen => 35.0,
            _ => todo!()
        })
    }
    pub fn rev_max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Zubr => 29.0,
            Self::Momi => 31.0,
            Self::Yasen => 21.0,
            _ => todo!()
        })
    }
    pub fn diving_speed(&self) -> Speed {
        Speed::from_raw(match self {
            Self::Zubr => 0.004,  // FIXME
            Self::Momi => 0.005,
            Self::Yasen => 0.004,
            _ => todo!()
        })
    }
    pub fn acceleration(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Zubr => 1.0,
            Self::Momi => 1.0,
            Self::Yasen => 1.0,
            _ => todo!()
        })
    }
    /// vec2(width, height)
    pub fn sprite_size(&self) -> Vec2 {
        (match self {
            Self::Momi => vec2(1024.0, 91.0),
            Self::Zubr => vec2(512.0, 190.0),
            Self::Yasen => vec2(1024.0, 156.0),
            _ => todo!()
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

#[derive(Debug, Component)]
pub struct BoatReversePositive;

#[derive(Debug, Component)]
pub struct BoatReverseNegative;

impl BoatReversePositive {
    pub fn relative_pos(circle_hud_radius: f32) -> Vec2 {
        Boat::MINIMUM_REVERSE.to_vec() * circle_hud_radius
    }
    pub fn mesh(length: f32) -> Segment2d {
        Segment2d::from_ray_and_length(
            Ray2d::new(Vec2::ZERO, Dir2::new(Boat::MINIMUM_REVERSE.to_vec()).unwrap()),
            length
        )
    }
}

impl BoatReverseNegative {
    pub fn relative_pos(circle_hud_radius: f32) -> Vec2 {
        (- Boat::MINIMUM_REVERSE).to_vec() * circle_hud_radius
    }
    pub fn mesh(length: f32) -> Segment2d {
        Segment2d::from_ray_and_length(
            Ray2d::new(Vec2::ZERO, Dir2::new((-Boat::MINIMUM_REVERSE).to_vec()).unwrap()),
            length
        )
    }
}
