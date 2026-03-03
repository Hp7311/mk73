use std::{f32::consts::PI, ops::{AddAssign, Neg}};

use bevy::prelude::*;

use crate::{DEFAULT_MAX_TURN_DEG, DEFAULT_SPRITE_SHRINK};

#[derive(Component, Debug, Copy, Clone, Default)]
pub(crate) struct CustomTransform {
    /// along the `rotation`
    pub(crate) speed: Speed,
    pub(crate) position: Position,
    /// stores the radian to move, with -> of Sprite as 0
    /// 
    /// ignores any reverse, calculates them like normal
    pub(crate) rotation: Radian,
    pub(crate) reversed: bool,
}

impl CustomTransform {
    pub(crate) fn rotate_local_z(&mut self, angle: Radian) {
        let rotation = angle.to_quat();
        self.rotation = (rotation * self.rotation.to_quat()).to_radian_unchecked();
    }
    /// from a not-moving entity
    pub(crate) fn from_static(position: Vec2) -> Self {
        CustomTransform {
            position: Position(position),
            ..default()
        }
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub(crate) struct Radius(pub(crate) f32);

impl Radius {
    pub(crate) fn default_convert(&self) -> Self {
        Radius(self.0 * DEFAULT_SPRITE_SHRINK)
    }
}
#[derive(Component, Debug, Copy, Clone, Default)]
pub(crate) struct Speed(pub(crate) f32);
#[derive(Component, Debug, Copy, Clone)]
pub(crate) struct MaxSpeed(pub(crate) f32);
#[derive(Component, Debug, Copy, Clone)]
pub(crate) struct ReverseSpeed(pub(crate) f32);

/// currently interpretated as maximum pixels of speed per frame
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Acceleration(pub(crate) f32);

#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct TargetRotation(pub(crate) Option<f32>);

impl From<Option<f32>> for TargetRotation {
    fn from(value: Option<f32>) -> Self {
        match value {
            Some(v) => TargetRotation(Some(v)),
            None => TargetRotation(None),
        }
    }
}

#[derive(Component, Debug, Copy, Clone, Default)]
pub(crate) struct TargetSpeed(pub(crate) f32);

impl From<f32> for TargetSpeed {
    fn from(value: f32) -> Self {
        TargetSpeed(value)
    }
}

#[derive(Component, Debug, Copy, Clone, Default)]
pub(crate) struct Radian(pub(crate) f32);

impl Radian {
    /// Replacement for [`f32::sin_cos`] (returns `vec2(cos, sin)`). Uses cross-platform
    /// deterministic sin/cos.
    pub(crate) fn to_vec(self) -> Vec2 {
        vec2(self.0.cos(), self.0.sin())
    }
}
impl Neg for Radian {
    type Output = Radian;
    fn neg(self) -> Self::Output {
        Radian(-self.0)
    }
}
pub(crate) trait ToRadian {
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
    pub(crate) fn from_deg(deg: f32) -> Self {
        Radian(deg.to_radians())
    }
    pub(crate) fn to_quat(self) -> Quat {
        Quat::from_rotation_z(self.0)
    }
}

#[derive(Component, Debug, PartialEq, Copy, Clone, Default)]
pub(crate) struct Position(pub(crate) Vec2);

impl AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Position {
    pub(crate) fn to_vec3(self) -> Vec3 {
        self.0.extend(0.0)
    }
}

#[derive(Bundle, Debug, Clone)]
pub(crate) struct ShipBundle {
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
    /// the target speed of the Ship
    target_speed: TargetSpeed,
    /// maximum speed acceleration per frame
    acceleration: Acceleration,
    /// stores dimension of the image once loaded
    dimensions: Dimensions,
}

impl ShipBundle {
    /// default to rotated 90 degrees and 2.0 turning
    pub(crate) fn new(
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
            target_speed: TargetSpeed(0.0),
            reverse_released: ReleasedAfterReverse(false),
            acceleration: Acceleration(acceleration),
            dimensions: Dimensions(None)
        }
    }
}

#[derive(Bundle, Debug, Clone)]
pub(crate) struct CircleHudBundle {
    pub(crate) mesh: Mesh2d,
    pub(crate) materials: MeshMaterial2d<ColorMaterial>,
}

/// # Example
/// Zero = 1.0,
/// Two = 0.01
#[allow(dead_code)]
pub(crate) enum DecimalPoint {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
}

/// the ship will reverse to any angle if LMB is held down after reversing.
/// 
/// once released, mouse being in the forward zone will be interpretated as forwards
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct ReleasedAfterReverse(pub(crate) bool);


/// flips a radian 180 degrees
pub(crate) trait FlipRadian {
    fn flip(self) -> Self;
}

impl FlipRadian for f32 {
    fn flip(self) -> Self {
        (self + PI).trim()
    }
}

/// eliminates offset when turning over the <--- axis
pub(crate) trait TrimRadian {
    fn trim(self) -> Self;
}
impl TrimRadian for f32 {
    fn trim(mut self) -> Self {
        if self > PI {
            self -= 2.0 * PI;
        } else if self < -PI {
            self += 2.0 * PI;
        }
        self
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Dimensions(pub(crate) Option<WidthHeight>);

#[derive(Component, Debug, Copy, Clone)]
pub(crate) struct WidthHeight {
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl WidthHeight {
    pub(crate) const ZERO: Self = WidthHeight { width: 0.0, height: 0.0 };
    pub(crate) fn to_rect(self, center_pos: Vec2) -> Rect {
        Rect::from_center_size(center_pos, vec2(self.width, self.height))
    }
    pub(crate) fn to_vec2(self) -> Vec2 {
        vec2(self.width, self.height)
    }
}

impl From<Vec2> for WidthHeight {
    fn from(value: Vec2) -> Self {
        WidthHeight { width: value.x, height: value.y }
    }
}

pub(crate) trait RectIntersect {
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
    use crate::ship::CircleHud;

    use super::*;
    #[test]
    fn test_flip() {
        let src = 80.0f32.to_radians();
        let expected = -100.0f32.to_radians();

        assert!((src.flip() - expected).abs() < 0.1);
    }
    #[test]
    fn test_circle_hud() {
        let circle_hud = CircleHud { radius: 3.0, center: vec2(0., 0.)};

        let target = vec2(2.8, 0.0);

        assert!(circle_hud.contains(target))
    }
}
