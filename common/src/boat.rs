use bevy::prelude::*;
use lightyear::core::id::PeerId;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter, VariantArray};

use crate::primitives::{Radian, Size};
use crate::{
    DEFAULT_MAX_TURN_DEG,
    primitives::Speed
};
use macros::{BoatImpl, FetchSprite, MaxSpeed, Size};


/// for performance improvements in diving
#[derive(Resource, PartialEq)]
#[cfg(feature = "client")]
pub struct BoatType(pub SubKind);

#[derive(BoatImpl, FetchSprite, Size, MaxSpeed, EnumCount, EnumIter, VariantArray, Component, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Boat {
    #[armanents(Mark18, 2, default)]
    #[armanents(Shell_57x441Mmr, 2)]
    #[armanents(Mark9, 4)]
    #[max_speed = 31]
    #[level = 1]
    #[length = 35]
    FairmileD,
    #[armanents(Type53, 2, default)]
    #[max_speed = 53]
    #[level = 1]
    #[length = 18.9]
    G5,
    #[armanents(Type53, 2, default)]
    #[armanents(Shell_57x441Mmr, 2)]
    #[armanents(Mark9, 4)]
    #[max_speed = 43.9]
    #[level = 1]
    #[length = 25.4]
    Komar,
    #[armanents(None)]
    #[max_speed = 31.1]
    #[level = 1]
    #[length = 36.9]
    Olympias,
    #[armanents(Mark18, 4, default)]
    #[max_speed = 40.9]
    #[level = 1]
    #[length = 23]
    Pt34,
    #[armanents(Mark18, 4, default)]
    #[armanents(Shell_127x680Mmr, 3)]
    #[armanents(Mark9, 4)]
    #[max_speed = 36]
    #[level = 2]
    #[length = 85.3]
    Momi,
    #[armanents(Mark18, 5, default)]
    #[armanents(Shell_57x441Mmr, 1)]
    #[max_speed = 17.6]
    #[level = 2]
    #[length =  67.1]
    TypeViic,
    #[armanents(Of45, 9, default)]
    #[armanents(Shell_25x129Mmr, 2)]
    #[max_speed = 55]
    #[level = 2]
    #[length = 57]
    Zubr,
    #[armanents(Set65, 4, default)]  // or maybe 6?
    #[armanents(BrahMos, 4)]
    #[armanents(Vodopad, 4)]
    #[armanents(Igla, 2)]
    #[armanents(Brosok, 4)]
    #[max_speed = 35]
    #[level = 8]
    #[length = 130]
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
    pub const VARIANTS: &'static [Self] = <Self as VariantArray>::VARIANTS;
    pub fn sub_kind(&self) -> SubKind {
        use SubKind::*;
        match self {
            Self::FairmileD => SurfaceShip,
            Self::G5 => SurfaceShip,
            Self::Komar => SurfaceShip,
            Self::Olympias => SurfaceShip, // hmm
            Self::Pt34 => SurfaceShip,
            Self::Momi => SurfaceShip,
            Self::TypeViic => Submarine,
            Self::Zubr => HoverCraft,
            Self::Yasen => Submarine
        }
    }
    pub fn rev_max_speed(&self) -> Speed {
        self.max_speed() * 0.6
    }
    pub fn diving_speed(&self) -> Speed {
        if self.sub_kind() == SubKind::Submarine {
            Speed::from_raw(0.004)
        } else {
            unreachable!()
        }
    }
    pub fn acceleration(&self) -> Speed {
        Speed::from_knots(1.0)
    }
    /// max turn in degrees
    pub fn max_turn(&self) -> Radian {
        DEFAULT_MAX_TURN_DEG
    }
    /// radius in pixels (length)
    pub fn radius(&self) -> f32 {
        self.render_size().x / 2.0
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
