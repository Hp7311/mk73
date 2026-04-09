use std::{
    f32::consts::PI,
    ops::{AddAssign, Neg},
};

use bevy::{prelude::*, sprite_render::Material2d};
use serde::{Deserialize, Serialize};

use crate::{boat::Boat, weapon::Weapon};

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
    }
    /// from a not-moving entity
    pub fn from_static(position: Vec2) -> Self {
        CustomTransform {
            position: Position(position),
            ..default()
        }
    }
}
/// helper struct for accessing the [`Boat`]'s circle HUD
#[derive(Debug, Component, Copy, Clone)]
pub struct CircleHud {
    pub radius: f32,
    pub center: Vec2,
}

impl CircleHud {
    /// whether `point` is in the Circle HUD
    pub(crate) fn contains(&self, point: Vec2) -> bool {
        point.distance_squared(self.center) < self.radius.powi(2)
    }
    /// whether a point is at HUD's center
    ///
    /// adjusted for decimal-point precision
    pub(crate) fn at_center(&self, point: Vec2, decimal_point: DecimalPoint) -> bool {
        let x_diff = (point.x - self.center.x).abs();
        let y_diff = (point.y - self.center.y).abs();

        x_diff < decimal_point.to_f32() && y_diff < decimal_point.to_f32()
    }
}

#[derive(Debug, Component, Clone)]
pub struct WeaponCounter {
    pub aval_weapons: Vec<Weapon>, // FIXME and maybe HashMap<Weapon, u16>
    pub selected_weapon: Option<Weapon>, // potential terry fox
}

#[derive(Debug, Clone, Copy)]
pub struct MkRect {
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
pub struct Speed(f32);

impl Speed {
    pub fn from_knots(knots: f32) -> Self {
        Speed(knots / 23.0)
    }
    pub fn from_raw(raw: f32) -> Self {
        Speed(raw)
    }
    pub fn add_raw(&mut self, raw: f32) {
        self.0 += raw;
    }
    pub fn subtract_raw(&mut self, raw: f32) {
        self.0 -= raw;
    }
    pub fn get_knots(&self) -> f32 {
        self.0 * 23.0
    }
    pub fn get_raw(&self) -> f32 {
        self.0
    }
    pub fn overwrite_with_raw(&mut self, raw: f32) {
        self.0 = raw
    }
}

/// the direction by which the ship should aim to turn towards
#[derive(Component, Debug, Clone, Copy, Default, Deref)]
pub struct TargetRotation(pub Option<f32>);

/// the target speed by which the ships should aim to accelerate towards
#[derive(Component, Debug, Copy, Clone, Default, Deref)]
pub struct TargetSpeed(pub Speed);

/// Used by [`CustomTransform`] for rotation
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Deref, Component, PartialEq)]
pub struct Radian(pub f32);

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
pub trait ToRadian {
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
    pub fn from_deg(deg: f32) -> Self {
        Radian(deg.to_radians())
    }
    pub fn to_quat(self) -> Quat {
        Quat::from_rotation_z(self.0)
    }
}

#[derive(Component, Debug, PartialEq, Copy, Clone, Default, Deref)]
pub struct Position(pub Vec2);

impl AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Position {
    pub fn to_vec3(self, z_index: f32) -> Vec3 {
        self.0.extend(z_index)
    }
}
impl From<Vec2> for Position {
    fn from(value: Vec2) -> Self {
        Self(value)
    }
}

#[derive(Debug, Resource, Clone, Copy, Default)]
pub struct CursorPos(pub Vec2);

/// the altitude of an entity
pub trait Altitude {
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
pub struct OutOfBound(pub bool);

/// ### Example
/// ```rs,norun
/// // params
/// mut meshes: ResMut<Assets<Mesh>>,
/// mut materials: ResMut<Assets<ColorMaterial>>
///
/// commands.spawn(MeshBundle {
///     mesh: Mesh2d(meshes.add(Circle::new(3.0))),
///     materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(RED)))
/// });
/// ```
#[derive(Bundle, Debug, Clone)]
pub struct MeshBundle<M: Material2d> {
    pub mesh: Mesh2d,
    pub materials: MeshMaterial2d<M>,
}

/// used for non-precise `==` comparisons
///
/// # Example
/// Zero = 1.0,
/// Two = 0.01
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum DecimalPoint {
    Zero,
    One,
    Two,
    Three,
}

impl DecimalPoint {
    pub fn to_f32(&self) -> f32 {
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
pub trait FlipRadian {
    fn flip(self) -> Self;
}

impl FlipRadian for f32 {
    fn flip(self) -> Self {
        (self + PI).normalize()
    }
}

/// eliminates offset when turning over the negative x-axis
pub trait NormalizeRadian {
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
pub struct WidthHeight {
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

// #[cfg(test)]
// mod tests {
//     use crate::{boat::CircleHud, util::eq};

//     use super::*;
//     #[test]
//     fn test_flip() {
//         let src = 80.0f32.to_radians();
//         let expected = -100.0f32.to_radians();

//         assert!(eq(src.flip(), expected, DecimalPoint::Three));
//     }
//     #[test]
//     fn test_circle_hud() {
//         let circle_hud = CircleHud {
//             radius: 3.0,
//             center: vec2(0., 0.),
//         };

//         let target = vec2(2.8, 0.0);

//         assert!(circle_hud.contains(target))
//     }
//     #[test]
//     fn test_mkrect() {
//         let rect = MkRect {
//             center: vec2(0.0, 0.0),
//             dimensions: WidthHeight::splat(10.0),
//         };

//         let expected = [
//             vec2(-5.0, 5.0),
//             vec2(5.0, 5.0),
//             vec2(5.0, -5.0),
//             vec2(-5.0, -5.0),
//         ];

//         assert_eq!(rect.get_corners(), expected);
//     }
// }
