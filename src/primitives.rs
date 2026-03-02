use std::{f32::consts::PI, ops::{AddAssign, Neg}};

use bevy::prelude::*;

use crate::{constants::{DEFAULT_MAX_TURN_DEG, DEFAULT_SPRITE_SHRINK}, ship::{WORLD_EXPAND, WORLD_MIN}, util::{TrimRadian, get_map_size}};

#[derive(Component, Debug, Copy, Clone, Default)]
pub struct CustomTransform {
    /// along the `rotation`
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
    /// from a not-moving entity
    pub fn from_static(position: Vec2) -> Self {
        CustomTransform {
            position: Position(position),
            ..default()
        }
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Radius(pub f32);

impl Radius {
    pub fn default_convert(&self) -> Self {
        Radius(self.0 * DEFAULT_SPRITE_SHRINK)
    }
}
#[derive(Component, Debug, Copy, Clone, Default)]
pub struct Speed(pub f32);
#[derive(Component, Debug, Copy, Clone)]
pub struct MaxSpeed(pub f32);
#[derive(Component, Debug, Copy, Clone)]
pub struct ReverseSpeed(pub f32);

/// currently interpretated as maximum pixels of speed per frame
#[derive(Component, Debug, Clone, Copy)]
pub struct Acceleration(pub f32);

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

#[derive(Component, Debug, Copy, Clone, Default)]
pub struct Radian(pub f32);

impl Radian {
    /// Replacement for [`f32::sin_cos`] (returns `vec2(cos, sin)`). Uses cross-platform
    /// deterministic sin/cos.
    pub fn to_vec(self) -> Vec2 {
        vec2(self.0.cos(), self.0.sin())
    }
}
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
        let (.., z) = self.to_euler(EulerRot::XYZ);
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

#[derive(Component, Debug, PartialEq, Copy, Clone, Default)]
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

#[derive(Bundle, Debug, Clone)]
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
    /// ship's sprite
    sprite: Sprite,
    /// whether reversed, speed etc
    custom_transform: CustomTransform,
    /// if reversed, whether LMB has been released since reversing
    reverse_released: ReleasedAfterReverse,
    /// raw image radius
    radius: Radius,
    /// where the user's mouse was facing
    mouse_target: TargetRotation,
    /// maximum speed acceleration per frame
    acceleration: Acceleration,
    /// stores dimension of the image once loaded
    dimensions: Dimensions,
}

impl ShipBundle {
    /// default to rotated 90 degrees and 2.0 turning
    pub fn new(
        max_speed: f32,
        reverse_speed: f32,
        acceleration: f32,
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
            reverse_released: ReleasedAfterReverse(false),
            acceleration: Acceleration(acceleration),
            dimensions: Dimensions(None)
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

/// the ship will reverse to any angle if LMB is held down after reversing.
/// 
/// once released, mouse being in the forward zone will be interpretated as forwards
#[derive(Component, Debug, Clone, Copy)]
pub struct ReleasedAfterReverse(pub bool);



#[derive(Component, Debug, Copy, Clone)]
pub struct WorldSize(pub WidthHeight);

impl Default for WorldSize {
    fn default() -> Self {
        WorldSize(get_map_size(1, WORLD_MIN, WORLD_EXPAND).into())
    }
}

/// flips a radian 180 degrees
pub trait FlipRadian {
    fn flip(self) -> Self;
}

impl FlipRadian for f32 {
    fn flip(self) -> Self {
        (self + PI).trim()
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Dimensions(pub Option<WidthHeight>);

#[derive(Component, Debug, Copy, Clone)]
pub struct WidthHeight {
    pub width: f32,
    pub height: f32,
}

impl WidthHeight {
    pub fn to_rect(&self, center_pos: Vec2) -> Rect {
        Rect::from_center_size(center_pos, vec2(self.width, self.height))
    }
    pub fn to_vec2(self) -> Vec2 {
        vec2(self.width, self.height)
    }
}

impl From<Vec2> for WidthHeight {
    fn from(value: Vec2) -> Self {
        WidthHeight { width: value.x, height: value.y }
    }
}

pub trait RectIntersect {
    fn intersects_with(&self, rhs: &Self) -> bool;
    fn x(&self) -> f32;
    fn y(&self) -> f32;
    fn right(&self) -> f32;
    fn bottom(&self) -> f32;
}

impl RectIntersect for Rect {
    fn intersects_with(&self, rhs: &Self) -> bool {
        self.x() < rhs.right()
            && self.right() > rhs.x()
            && self.y() < rhs.bottom()
            && self.bottom() > rhs.y()
    }
    
    fn x(&self) -> f32 {
        self.min.x
    }
    
    fn y(&self) -> f32 {
        self.min.y
    }
    
    fn right(&self) -> f32 {
        self.max.x
    }
    
    fn bottom(&self) -> f32 {
        self.max.y
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_flip() {
        let src = 80.0f32.to_radians();
        let expected = -100.0f32.to_radians();

        assert!((src.flip() - expected).abs() < 0.1);
    }
}
