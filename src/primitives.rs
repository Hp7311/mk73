use std::ops::{AddAssign, Neg};

use bevy::prelude::*;

use crate::constants::{DEFAULT_MAX_TURN_DEG, DEFAULT_SPRITE_SHRINK};

#[derive(Component, Debug, Copy, Clone)]
pub struct CustomTransform {
    pub speed: Speed,
    pub position: Position,
    /// stores the radian to move, with -> of Sprite as 0
    /// 
    /// ignores any reverse, calculates them like normal
    pub rotation: Radian,
    pub reversed: bool,
}

impl CustomTransform {
    pub fn rotate_local_z(&mut self, angle: Radian) {
        let rotation = angle.to_quat();
        self.rotation = (rotation * self.rotation.to_quat()).to_radian_unchecked();
    } // TODO test
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Radius(pub f32);

impl Radius {
    pub fn default_convert(&self) -> Self {
        Radius(self.0 * DEFAULT_SPRITE_SHRINK)
    }
}
#[derive(Component, Debug, Copy, Clone)]
pub struct Speed(pub f32);
#[derive(Component, Debug, Copy, Clone)]
pub struct MaxSpeed(pub f32);
#[derive(Component, Debug, Copy, Clone)]
pub struct ReverseSpeed(pub f32);

#[derive(Component, Debug, Clone, Copy)]
pub struct TargetRotation(pub Option<f32>);

impl From<Option<f32>> for TargetRotation {
    fn from(value: Option<f32>) -> Self {
        match value {
            Some(v) => TargetRotation(Some(v)),
            None => TargetRotation(None),
        }
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Radian(pub f32);

impl Neg for Radian {
    type Output = Radian;
    fn neg(self) -> Self::Output {
        Radian(-self.0)
    }
}
pub trait ToRadian {
    fn to_radian_unchecked(&self) -> Radian;
}

impl ToRadian for f32 {
    /// assumes already radian
    fn to_radian_unchecked(&self) -> Radian {
        Radian(*self)
    }
}
impl ToRadian for Quat {
    fn to_radian_unchecked(&self) -> Radian {
        let (_, _, z) = self.to_euler(EulerRot::XYZ);
        Radian(z)
    }
}
impl Radian {
    pub fn from_deg(deg: f32) -> Self {
        Radian(deg.to_radians())
    }
    pub fn to_quat(self) -> Quat {
        Quat::from_rotation_z(self.0)
    }
}

#[derive(Component, Debug, PartialEq, Copy, Clone)]
pub struct Position(pub Vec2);

impl AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Position {
    pub fn to_vec3(self) -> Vec3 {
        self.0.extend(0.0)
    }
}

#[derive(Bundle, Debug)]
pub struct ShipBundle {
    /// maximum angle in radians that you can turn per frame, consider deriving from `max_speed`
    /// ### Warning
    /// keep the value small
    max_turn: Radian,
    /// max speed that the Ship can have
    max_speed: MaxSpeed,
    reverse_speed: ReverseSpeed,
    /// tranform to update in seperate system
    transform: Transform,
    sprite: Sprite,
    custom_transform: CustomTransform,
    /// raw image
    radius: Radius,
    /// where the user's mouse is facing
    mouse_target: TargetRotation,
}

impl ShipBundle {
    /// default to rotated 90 degrees and 2.0 turning
    pub fn new(
        max_speed: f32,
        reverse_speed: f32,
        position: Vec2,
        sprite_name: &str,
        asset_server: AssetServer,
        radius: f32,
    ) -> Self {
        const SPRITE_ROTATION: f32 = 90.0;
        let sprite = Sprite::from_image(asset_server.load(sprite_name.to_owned()));

        let transform = Transform {
            translation: position.extend(0.0),
            rotation: Quat::from_rotation_z(SPRITE_ROTATION.to_radians()),
            ..default()
        };

        ShipBundle {
            max_turn: Radian::from_deg(DEFAULT_MAX_TURN_DEG),
            max_speed: MaxSpeed(max_speed),
            reverse_speed: ReverseSpeed(reverse_speed),
            transform,
            sprite,
            radius: Radius(radius),
            custom_transform: CustomTransform {
                speed: Speed(0.0),
                position: Position(position),
                rotation: Radian::from_deg(SPRITE_ROTATION),
                reversed: false
            },
            mouse_target: None.into(),
        }
    }
}

#[derive(Bundle, Debug, Clone)]
pub struct CircleHudBundle {
    pub mesh: Mesh2d,
    pub materials: MeshMaterial2d<ColorMaterial>,
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Background;

#[derive(Component, Debug, Copy, Clone)]
pub struct OilRig;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectWithWh {
    pub pos: Vec2,
    pub w_h: Vec2,
}

impl RectWithWh {
    pub fn intersects_with(&self, rhs: &Self) -> bool {
        self.x() < rhs.right()
            && self.right() > rhs.x()
            && self.y() < rhs.bottom()
            && self.bottom() > rhs.y()
    }

    pub fn x(&self) -> f32 {
        self.pos.x
    }
    pub fn y(&self) -> f32 {
        self.pos.y
    }
    pub fn right(&self) -> f32 {
        self.pos.x + self.w_h.x
    }

    pub fn bottom(&self) -> f32 {
        self.pos.y + self.w_h.y
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct WorldSize(pub Vec2);
