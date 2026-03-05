use std::{
    f32::consts::PI,
    ops::{AddAssign, Neg},
};

use bevy::prelude::*;

use crate::{DEFAULT_MAX_TURN_DEG, WATER_SURFACE};

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

#[derive(Component, Debug, Copy, Clone, Deref)]
pub(crate) struct Radius(pub f32);

#[derive(Component, Debug, Copy, Clone, Default)]
pub(crate) struct Speed(f32);

impl Speed {
    pub(crate) fn from_knots(knots: f32) -> Self {
        Speed(knots / 23.0)
    }
    pub(crate) fn from_raw(raw: f32) -> Self {
        Speed(raw)
    }
    pub(crate) fn add_raw(&mut self, raw: f32) {
        self.0 += raw;
    }
    pub(crate) fn subtract_raw(&mut self, raw: f32) {
        self.0 -= raw;
    }
    pub(crate) fn get_knots(&self) -> f32 {
        self.0 * 23.0
    }
    pub(crate) fn get_raw(&self) -> f32 {
        self.0
    }
}

#[derive(Component, Debug, Copy, Clone, Default, Deref)]
pub(crate) struct MaxSpeed(pub Speed);
#[derive(Component, Debug, Copy, Clone, Default, Deref)]
pub(crate) struct ReverseSpeed(pub Speed);

/// currently interpretated as maximum pixels of speed per frame
#[derive(Component, Debug, Clone, Copy, Default, Deref)]
pub(crate) struct Acceleration(pub Speed);

#[derive(Component, Debug, Clone, Copy, Default, Deref)]
pub(crate) struct TargetRotation(pub Option<f32>);

impl From<Option<f32>> for TargetRotation {
    fn from(value: Option<f32>) -> Self {
        match value {
            Some(v) => TargetRotation(Some(v)),
            None => TargetRotation(None),
        }
    }
}

#[derive(Component, Debug, Copy, Clone, Default, Deref)]
pub(crate) struct TargetSpeed(pub Speed);

#[derive(Component, Debug, Copy, Clone, Default, Deref)]
pub(crate) struct Radian(pub f32);

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

#[derive(Component, Debug, PartialEq, Copy, Clone, Default, Deref)]
pub(crate) struct Position(pub Vec2);

impl AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Position {
    pub(crate) fn to_vec3(self, z_index: f32) -> Vec3 {
        self.0.extend(z_index)
    }
}
/// the altitude of an entity, with 0 being the surface and going up with increasing
///
/// implemented as a trait to sync with [`Transform::translation`]'s `z`-ordering
// TODO
pub(crate) trait Altitude {}

#[derive(Bundle, Debug, Clone)]
pub(crate) struct BoatBundle {
    /// maximum angle in radians that you can turn per frame, consider deriving from `max_speed`
    /// ### Warning
    /// keep the value small
    max_turn: Radian,
    /// max speed that the Boat can have
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
    /// the target speed of the Boat
    target_speed: TargetSpeed,
    /// maximum speed acceleration per frame
    acceleration: Acceleration,
}

impl BoatBundle {
    /// all Speeds are in knots
    ///
    /// radius derived
    pub(crate) fn new(
        max_speed: f32,
        reverse_speed: f32,
        acceleration: f32,
        position: Vec2,
        sprite: Sprite
    ) -> Self {
        const SPRITE_ROTATION: f32 = 90.0;
        assert!(sprite.custom_size.is_some());

        let transform = Transform {
            translation: position.extend(WATER_SURFACE),
            rotation: Quat::from_rotation_z(SPRITE_ROTATION.to_radians()),
            ..default()
        };

        println!("Radius: {}", sprite.custom_size.unwrap().x / 2.0);
        BoatBundle {
            max_turn: Radian::from_deg(DEFAULT_MAX_TURN_DEG),
            max_speed: MaxSpeed(Speed::from_knots(max_speed)),
            reverse_speed: ReverseSpeed(Speed::from_knots(reverse_speed)),
            transform,
            radius: Radius(sprite.custom_size.unwrap().x / 2.0),
            sprite,
            custom_transform: CustomTransform {
                speed: Speed(0.0),
                position: Position(position),
                rotation: Radian::from_deg(SPRITE_ROTATION),
                reversed: false,
            },
            mouse_target: None.into(),
            target_speed: TargetSpeed(Speed::from_knots(0.0)),
            reverse_released: ReleasedAfterReverse(false),
            acceleration: Acceleration(Speed::from_knots(acceleration)),
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
#[derive(Component, Debug, Clone, Copy, Deref)]
pub(crate) struct ReleasedAfterReverse(pub bool);

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

#[derive(Component, Debug, Copy, Clone)]
pub(crate) struct WidthHeight {
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl WidthHeight {
    pub(crate) const ZERO: Self = WidthHeight {
        width: 0.0,
        height: 0.0,
    };
    pub(crate) fn to_rect(self, center_pos: Vec2) -> Rect {
        Rect::from_center_size(center_pos, vec2(self.width, self.height))
    }
    pub(crate) fn to_vec2(self) -> Vec2 {
        vec2(self.width, self.height)
    }
    pub(crate) fn splat(num: f32) -> Self {
        WidthHeight {
            width: num,
            height: num,
        }
    }
}

impl From<Vec2> for WidthHeight {
    fn from(value: Vec2) -> Self {
        WidthHeight {
            width: value.x,
            height: value.y,
        }
    }
}

/// used to indicate that an entity (usually a `Sprite`) is validated to prevent reduntancy
#[derive(Debug, Component, Clone, Copy, Deref)]
pub(crate) struct Validated(pub bool);

#[cfg(test)]
mod tests {
    use crate::boat::CircleHud;

    use super::*;
    #[test]
    fn test_flip() {
        let src = 80.0f32.to_radians();
        let expected = -100.0f32.to_radians();

        assert!((src.flip() - expected).abs() < 0.1);
    }
    #[test]
    fn test_circle_hud() {
        let circle_hud = CircleHud {
            radius: 3.0,
            center: vec2(0., 0.),
        };

        let target = vec2(2.8, 0.0);

        assert!(circle_hud.contains(target))
    }
}
