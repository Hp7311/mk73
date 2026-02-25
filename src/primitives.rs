use std::ops::{AddAssign, Neg};

use bevy::prelude::*;

use crate::constants::DEFAULT_MAX_TURN_DEG;

#[derive(Component, Debug)]
pub struct CustomTransform {
    pub speed: Speed,
    pub position: Position,
    pub rotation: Radian,
}

impl CustomTransform {
    pub fn rotate_local_z(&mut self, angle: Radian) {
        let rotation = angle.to_quat();
        self.rotation = (rotation * self.rotation.to_quat()).to_radian_unchecked();
    }  // TODO test
}

#[derive(Component, Debug)]
pub struct Radius(pub f32);

impl Radius {
    pub fn default_convert(&self) -> Self {
        Radius(self.0 * 0.3)
    }
    pub fn custom_convert(&self, ratio: f32) -> Self {
        Radius(self.0 * ratio)
    }
}
#[derive(Component, Debug)]
pub struct Speed(pub f32);

#[derive(Component, Debug)]
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
    pub fn to_quat(&self) -> Quat {
        Quat::from_rotation_z(self.0)
    }
}

#[derive(Component, Debug, PartialEq)]
pub struct Position(pub Vec2);

impl AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Position {
    pub fn to_vec3(&self) -> Vec3 {
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
    max_speed: Speed,
    /// tranform to update in seperate system
    transform: Transform,
    sprite: Sprite,
    custom_transform: CustomTransform,
    /// raw image
    radius: Radius,
}

impl ShipBundle {
    /// default to rotated 90 degrees and 2.0 turning
    pub fn new(
        max_speed: f32,
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
            max_speed: Speed(max_speed),
            transform,
            sprite,
            radius: Radius(radius),
            custom_transform: CustomTransform {
                speed: Speed(0.0),
                position: Position(position),
                rotation: Radian::from_deg(SPRITE_ROTATION),
            }
        }
    }
}

#[derive(Bundle, Debug)]
pub struct CircleHudBundle {
    pub mesh: Mesh2d,
    pub materials: MeshMaterial2d<ColorMaterial>,
}
