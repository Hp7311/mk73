use bevy::{prelude::*, sprite_render::Material2d};
use serde::{Deserialize, Serialize};
use std::ops::Mul;
use std::{
    f32::consts::PI,
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
};
use std::collections::HashMap;
use bevy::math::FloatPow;
use rand::distr::StandardUniform;
use rand::prelude::Distribution;
use rand::{Rng, RngExt};
use crate::weapon::Weapon;
use crate::collision::out_of_bounds;
use crate::util::move_with_rotation;
use crate::world::WorldSize;

/// note that this is not updated on client for boats that it doesn't control
#[derive(Component, Debug, Copy, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct CustomTransform {
    /// along the `rotation`, negative if reversed
    pub speed: Speed,
    pub position: Position,
    /// stores the radian to move for the head of the boat, with -> of Sprite as 0
    pub rotation: Radian,
}

impl CustomTransform {
    pub fn rotate_local_z(&mut self, angle: Radian) {
        let rotation = angle.to_quat();
        self.rotation = (rotation * self.rotation.to_quat()).wrap_radian();
    }
    /// according to `self.rotation` and `self.speed`, move one frame
    pub fn move_position(&mut self) {
        self.position.0 += move_with_rotation(self.rotation, self.speed).xy();
    }
    /// same as [`move_position`] but with bound checking, returns true if success
    pub fn move_position_checked(&mut self, world_size: &WorldSize, sprite_size: Vec2) -> bool {
        let mut target = self.position.0;
        target += move_with_rotation(self.rotation, self.speed).xy();

        if out_of_bounds(
            world_size,
            Mk48Rect::new(target, sprite_size),
            self.rotation
        ) {
            false
        } else {
            self.position.0 = target;
            true
        }
        // consider slowing speed if out of bounds and decreasing health
    }
}

/// check if two points in range of each other by Pathogras theorem
#[inline]
pub fn in_range(first: Vec2, second: Vec2, by: f32) -> bool {
    Vec2::distance_squared(first, second) < by.squared()
}

#[derive(Debug, Component, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponCounter {
    pub weapons: HashMap<Weapon, u8>,
    pub selected_weapon: Option<Weapon> // potential terry fox
}

// TODO maybe Trait on Rect
/// useful helpers like getting corners and large bounding box
#[derive(Debug, Clone, Copy)]
pub struct Mk48Rect {
    pub center: Vec2,
    pub dimensions: WidthHeight,
}

impl Mk48Rect {
    /// only left-upper and right-bottom for when 4 is not necessary
    /// 
    /// ### DO NOT use for when rotation matter
    pub(crate) fn two_corners(&self) -> [Vec2; 2] {
        [
            vec2(
                self.center.x - self.dimensions.width / 2.0,
                self.center.y + self.dimensions.height / 2.0,
            ),
            vec2(
                self.center.x + self.dimensions.width / 2.0,
                self.center.y - self.dimensions.height / 2.0,
            )
        ]
    }
    /// all 4 corners
    #[allow(dead_code)]
    pub(crate) fn corners(&self) -> [Vec2; 4] {
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
    /// all 4 relative corners
    pub(crate) fn relative_corners(&self) -> impl Iterator<Item = Vec2> {
        [
            vec2(-self.dimensions.width / 2.0, self.dimensions.height / 2.0),
            vec2(self.dimensions.width / 2.0, self.dimensions.height / 2.0),
            vec2(self.dimensions.width / 2.0, -self.dimensions.height / 2.0),
            vec2(-self.dimensions.width / 2.0, -self.dimensions.height / 2.0),
        ].into_iter()
    }
    /// only left-upper and right-bottom for when 4 is not necessary
    /// 
    /// ### DO NOT use for when rotation matter
    #[allow(dead_code)]
    pub(crate) fn relative_two_corners(&self) -> impl Iterator<Item = Vec2> {
        [
            vec2(-self.dimensions.width / 2.0, self.dimensions.height / 2.0),
            vec2(self.dimensions.width / 2.0, -self.dimensions.height / 2.0)
        ].into_iter()
    }
    pub fn new(center: Vec2, dimensions: impl Into<WidthHeight>) -> Self {
        Mk48Rect {
            center,
            dimensions: dimensions.into()
        }
    }
    /// makes `dimensions` zero
    pub fn from_point(point: impl Into<Vec2>) -> Self {
        Mk48Rect {
            center: point.into(),
            dimensions: WidthHeight::ZERO
        }
    }
    pub(crate) fn contains(&self, pos: Vec2) -> bool {
        self.to_rect().contains(pos)
    }
    pub(crate) fn to_rect(self) -> Rect {
        Rect::from_center_size(self.center, self.dimensions.to_vec2())
    }
    /// creates a large bounding box that is guaranteed to contain self no matter the rotation
    pub(crate) fn large_bounding_box(mut self) -> Self {
        self.dimensions = WidthHeight::splat(self.dimensions.max_side() * WidthHeight::LARGE_BOX_MULTIPLIER);
        self
    }
}

/// helper struct containing a raw speed
///
/// all ops default to raw repensentation
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, Deref, Component, PartialEq, Reflect,
)]
pub struct Speed(f32);

impl Speed {
    pub const ZERO: Self = Self(0.0);
    pub fn from_knots(knots: f32) -> Self {
        Speed(knots / 23.0)
    }
    pub fn from_raw(raw: f32) -> Self {
        Speed(raw)
    }
    #[deprecated = "Use += instead"]
    pub fn add_raw(&mut self, raw: f32) {
        self.0 += raw;
    }
    #[deprecated = "Use -= instead"]
    pub fn subtract_raw(&mut self, raw: f32) {
        self.0 -= raw;
    }
    pub fn get_knots(&self) -> f32 {
        self.0 * 23.0
    }
    pub fn get_raw(&self) -> f32 {
        self.0
    }
    /// with raw
    pub fn overwrite(&mut self, with: Speed) {
        *self = with;
    }
}

impl Sub for Speed {
    type Output = Speed;
    fn sub(self, rhs: Self) -> Self::Output {
        Speed::from_raw(self.0 - rhs.0)
    }
}
impl SubAssign for Speed {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl Add for Speed {
    type Output = Speed;
    fn add(self, rhs: Self) -> Self::Output {
        Speed::from_raw(self.0 + rhs.0)
    }
}
impl AddAssign for Speed {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Neg for Speed {
    type Output = Speed;
    fn neg(self) -> Self::Output {
        Speed::from_raw(-self.0)
    }
}
impl PartialOrd for Speed {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

/// the direction by which the ship should aim to turn towards
#[derive(Component, Debug, Clone, Copy, Default, Deref)]
pub struct TargetRotation(pub Radian);


/// used by weapons to find accleration
#[derive(Component, Copy, Clone)]
pub struct LastSpeed(pub Speed);

/// the target speed by which the ships should aim to accelerate towards
#[derive(Component, Debug, Copy, Clone, Default, Deref)]
pub struct TargetSpeed(pub Speed);

/// Used by [`CustomTransform`] for rotation
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Component, PartialEq, Reflect, PartialOrd, )]
pub struct Radian(pub f32);

impl Radian {
    /// multiply return type by the length to find the coordinates of a point
    /// ### Example
    /// ```ignore
    /// # use common::primitives::Radian;
    /// # use bevy::prelude::vec2;
    /// let angle = Radian::from_deg(45.0);
    /// assert_eq!(angle.to_vec() * 18.0f32.sqrt(), vec2(3.0, 3.0));  // approximate
    /// ```
    pub fn to_vec(self) -> Vec2 {
        vec2(self.0.cos(), self.0.sin())
    }
    /// normalizing and rotating
    pub fn rotate_local_z(&mut self, angle: Radian) {
        *self = Radian(self.0 + angle.0).normalize();
    }
    // test
    /// normalizing and rotating
    pub fn rotate_local_z_ret(&self, angle: Radian) -> Self {
        Radian(self.0 + angle.0).normalize()
    }
    pub const fn from_deg(deg: f32) -> Self {
        Radian(deg.to_radians())
    }
    pub fn to_quat(self) -> Quat {
        Quat::from_rotation_z(self.0)
    }
    pub fn to_degrees(self) -> f32 {
        self.0.to_degrees()
    }
    pub fn abs(self) -> Self {
        Radian(self.0.abs())
    }
}

impl Distribution<Radian> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Radian {
        Radian(rng.random())
    }
}
impl Neg for Radian {
    type Output = Radian;
    fn neg(self) -> Self::Output {
        Radian(-self.0)
    }
}

impl Mul<f32> for Radian {
    type Output = Self;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self.0 *= rhs;
        self
    }
}

impl Sub for Radian {
    type Output = Radian;
    fn sub(self, rhs: Self) -> Self::Output {
        Radian(self.0 - rhs.0)
    }
}

impl Add for Radian {
    type Output = Radian;
    fn add(self, rhs: Self) -> Self::Output {
        Radian(self.0 + rhs.0)
    }
}

pub trait WrapRadian {
    fn wrap_radian(&self) -> Radian;
}

impl WrapRadian for f32 {
    /// assumes already radian, wraps [`f32`] by [`Radian()`]
    fn wrap_radian(&self) -> Radian {
        Radian(*self)
    }
}
impl WrapRadian for Quat {
    /// takes the Z-rotation and wraps it in [`f32`]
    fn wrap_radian(&self) -> Radian {
        let (.., z) = self.to_euler(EulerRot::XYZ);
        Radian(z)
    }
}

impl WrapRadian for Radian {
    fn wrap_radian(&self) -> Radian {
        *self
    }
}

/// only used by the [`Boat`] entity to indicate the depth without bloating [`CustomTransform`] as a Component currently,  
/// also tricky because [`CustomTransform`] is both locally predicted and remotely while [`ZIndex`] should be client-authoritive
/// 
/// ### Important
/// only for the physics depth, NOT the rendering depth ([`Transform::translation`])
/// 
/// also used for strong typing
#[derive(Component, Serialize, Deserialize, PartialEq, Deref, PartialOrd, Copy, Clone, Debug, Default)]
pub struct ZIndex(pub f32);

impl Sub for ZIndex {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Add for ZIndex {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl Neg for ZIndex {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
pub trait WrapZIndex {
    /// wraps in [`ZIndex`]
    fn wrap_z(self) -> ZIndex;
}

impl WrapZIndex for f32 {
    fn wrap_z(self) -> ZIndex {
        ZIndex(self)
    }
}

pub trait GetZIndex {
    fn z_index(self) -> ZIndex;
}

impl GetZIndex for Vec3 {
    fn z_index(self) -> ZIndex {
        self.z.wrap_z()
    }
}
#[derive(Component, Debug, PartialEq, Copy, Clone, Default, Deref, Deserialize, Serialize)]
pub struct Position(pub Vec2);

impl AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Position {
    /// ## NOT for rendering, only physics
    pub fn extend(self, z_index: ZIndex) -> Vec3 {
        self.0.extend(z_index.0)
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
    /// returns Z-index after diving
    fn decrease_with_limit(&mut self, meter: f32, limit: ZIndex) -> ZIndex;
    /// returns Z-index after surfacing
    fn increase_with_limit(&mut self, meter: f32, limit: ZIndex) -> ZIndex;
    fn reached(&self, target: ZIndex, precision: DecimalPoint) -> bool;
}

impl Altitude for Transform {
    fn decrease_with_limit(&mut self, meter: f32, limit: ZIndex) -> ZIndex {
        self.translation.z = (self.translation.z - meter).max(*limit);
        self.translation.z_index()
    }

    fn increase_with_limit(&mut self, meter: f32, limit: ZIndex) -> ZIndex {
        self.translation.z = (self.translation.z + meter).min(*limit);
        self.translation.z_index()
    }

    fn reached(&self, target: ZIndex, precision: DecimalPoint) -> bool {
        let diff = (*target - self.translation.z).abs();

        diff <= precision.to_f32()
    }
}

#[derive(Debug, Component, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OutOfBound(pub bool);

/// ### Example
/// ```rs,no_run
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

/// flips a radian 180 degrees along with normalizing
pub trait FlipRadian {
    fn flip(self) -> Self;
}

impl FlipRadian for f32 {
    fn flip(self) -> Self {
        (self + PI).normalize()
    }
}
impl FlipRadian for Radian {
    fn flip(self) -> Self {
        Radian(self.0.flip())
    }
}

/// eliminates offset when turning over the negative x-axis
pub trait NormalizeRadian {
    /// normalize a radian within range `-PI..PI`
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
impl NormalizeRadian for Radian {
    fn normalize(self) -> Self {
        self.0.normalize().wrap_radian()
    }
}


#[derive(Resource, Debug, Copy, Clone, Deserialize, Serialize, PartialEq)]
pub struct WidthHeight {
    pub width: f32,
    pub height: f32,
}

impl WidthHeight {
    pub(crate) const LARGE_BOX_MULTIPLIER: f32 = 1.3;
    pub const ZERO: Self = WidthHeight {
        width: 0.0,
        height: 0.0,
    };
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
}

impl From<Vec2> for WidthHeight {
    fn from(value: Vec2) -> Self {
        WidthHeight {
            width: value.x,
            height: value.y,
        }
    }
}