use std::{
    f32::consts::PI,
    ops::{AddAssign, Neg},
};

use bevy::{prelude::*, sprite_render::Material2d};

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
    #[allow(dead_code)]
    pub(crate) fn from_static(position: Vec2) -> Self {
        CustomTransform {
            position: Position(position),
            ..default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MkRect {
    pub center: Vec2,
    pub dimensions: WidthHeight,
}

#[allow(dead_code)]
impl MkRect {
    pub(crate) fn get_corners(&self) -> [Vec2; 4] {
        [
            vec2(
                self.center.x - self.dimensions.width / 2.0,
                self.center.y + self.dimensions.height / 2.0,
            ),
            vec2(
                self.center.x + self.dimensions.width / 2.0,
                self.center.y + self.dimensions.height / 2.0,
            ),
            vec2(
                self.center.x + self.dimensions.width / 2.0,
                self.center.y - self.dimensions.height / 2.0,
            ),
            vec2(
                self.center.x - self.dimensions.width / 2.0,
                self.center.y - self.dimensions.height / 2.0,
            ),
        ]
    }
    pub(crate) fn get_relative_corners(&self) -> [Vec2; 4] {
        [
            vec2(-self.dimensions.width / 2.0, self.dimensions.height / 2.0),
            vec2(self.dimensions.width / 2.0, self.dimensions.height / 2.0),
            vec2(self.dimensions.width / 2.0, -self.dimensions.height / 2.0),
            vec2(-self.dimensions.width / 2.0, -self.dimensions.height / 2.0),
        ]
    }
    pub(crate) fn width(&self) -> f32 {
        self.dimensions.width
    }
    pub(crate) fn height(&self) -> f32 {
        self.dimensions.height
    }
    pub(crate) fn new(center: Vec2, width: f32, height: f32) -> Self {
        MkRect {
            center,
            dimensions: WidthHeight { width, height },
        }
    }
    pub(crate) fn contains(&self, pos: Vec2) -> bool {
        self.to_rect().contains(pos)
    }
    pub(crate) fn to_rect(&self) -> Rect {
        Rect::from_center_size(self.center, self.dimensions.to_vec2())
    }
}

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
    pub(crate) fn overwrite_with_raw(&mut self, raw: f32) {
        self.0 = raw
    }
}

/// the direction by which the ship should aim to turn towards
#[derive(Component, Debug, Clone, Copy, Default, Deref)]
pub(crate) struct TargetRotation(pub Option<f32>);

/// the target speed by which the ships should aim to accelerate towards
#[derive(Component, Debug, Copy, Clone, Default, Deref)]
pub(crate) struct TargetSpeed(pub Speed);

/// Used by [`CustomTransform`] for rotation
#[derive(Component, Debug, Copy, Clone, Default, Deref)]
pub(crate) struct Radian(pub f32);

impl Radian {
    /// Vec2 to be added to Origin to rotate
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
    /// assumes already radian, wraps [`f32`] by Radian()
    fn to_radian_unchecked(&self) -> Radian {
        Radian(*self)
    }
}
impl ToRadian for Quat {
    /// takes the Z-rotation and wraps it in [`f32`]
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

#[derive(Debug, Resource, Clone, Copy, Default)]
pub(crate) struct CursorPos(pub Vec2);

/// the altitude of an entity
pub(crate) trait Altitude {
    fn decrease_with_limit(&mut self, meter: f32, limit: f32);
    fn increase_with_limit(&mut self, meter: f32, limit: f32);
    fn reached(&self, target: f32, precision: DecimalPoint) -> bool;
}

impl Altitude for Transform {
    fn decrease_with_limit(&mut self, meter: f32, limit: f32) {
        self.translation.z = (self.translation.z - meter).max(limit);
        // info!("Altitude: {}", self.translation.z);
    }

    fn increase_with_limit(&mut self, meter: f32, limit: f32) {
        self.translation.z = (self.translation.z + meter).max(limit);
    }

    fn reached(&self, target: f32, precision: DecimalPoint) -> bool {
        let diff = (target - self.translation.z).abs();

        diff <= precision.to_f32()
    }
}

#[derive(Debug, Component, Clone, Copy)]
pub(crate) struct OutOfBound(pub bool);

#[derive(Bundle, Debug, Clone)]
pub(crate) struct MeshBundle<M: Material2d> {
    pub(crate) mesh: Mesh2d,
    pub(crate) materials: MeshMaterial2d<M>,
}

/// used for non-precise `==` comparisons
///
/// # Example
/// Zero = 1.0,
/// Two = 0.01
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum DecimalPoint {
    Zero,
    One,
    Two,
    Three,
}

impl DecimalPoint {
    pub(crate) fn to_f32(&self) -> f32 {
        use DecimalPoint as D;
        match self {
            D::Zero => 1.0,
            D::One => 0.1,
            D::Two => 0.01,
            D::Three => 0.001,
        }
    }
}

/// flips a radian 180 degrees
pub(crate) trait FlipRadian {
    fn flip(self) -> Self;
}

impl FlipRadian for f32 {
    fn flip(self) -> Self {
        (self + PI).normalize()
    }
}

/// eliminates offset when turning over the negative x-axis
pub(crate) trait NormalizeRadian {
    fn normalize(self) -> Self;
}
impl NormalizeRadian for f32 {
    fn normalize(mut self) -> Self {
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
    pub width: f32,
    pub height: f32,
}

impl WidthHeight {
    pub(crate) const LARGE_BOX_MULTIPLIER: f32 = 1.3;
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
    pub(crate) fn max_side(&self) -> f32 {
        if self.width > self.height {
            self.width
        } else {
            self.height
        }
    }
    /// creates a large bounding box that is guaranteed to contain self no matter the rotation
    pub(crate) fn large_bounding_box(self) -> Self {
        Self::splat(self.max_side() * Self::LARGE_BOX_MULTIPLIER)
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

#[cfg(test)]
mod tests {
    use crate::{boat::CircleHud, util::eq};

    use super::*;
    #[test]
    fn test_flip() {
        let src = 80.0f32.to_radians();
        let expected = -100.0f32.to_radians();

        assert!(eq(src.flip(), expected, DecimalPoint::Three));
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
    #[test]
    fn test_mkrect() {
        let rect = MkRect {
            center: vec2(0.0, 0.0),
            dimensions: WidthHeight::splat(10.0),
        };

        let expected = [
            vec2(-5.0, 5.0),
            vec2(5.0, 5.0),
            vec2(5.0, -5.0),
            vec2(-5.0, -5.0),
        ];

        assert_eq!(rect.get_corners(), expected);
    }
}
