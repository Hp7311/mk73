//! utility functions independent to game

use std::collections::VecDeque;
use std::iter::FromFn;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::{Index, Range, RangeInclusive};
use std::slice;
use std::sync::LazyLock;
use bevy::ecs::schedule::ScheduleConfigs;
// remember high test coverage
use bevy::{math::ops::atan2, prelude::*};
#[cfg(feature = "server")]
#[cfg(false)]
use lightyear::websocket::server::Identity;
use serde::{Deserialize, Serialize};
use crate::primitives::{Radian, Speed};
use crate::primitives::{Mk48Rect, WidthHeight, ZIndex};

/// can just chain [`run_if`](IntoScheduleConfigs::run_if)s..
pub fn in_states_2<T: States>(first: T, second: T)  -> impl Fn(Res<State<T>>) -> bool {
    move |state| *state.get() == first || *state.get() == second
}

pub fn not_in_state<T: States>(not: T) -> impl Fn(Res<State<T>>) -> bool {
    move |state| *state.get() != not
}
/// defaults to 0.001 precision
#[macro_export]
macro_rules! eq {
    ($x:expr, $y:expr) => {
        ($x - $y).abs() < 0.001
    };
    ($x:expr, $y:expr, ?precision = $precision:expr) => {
        ($x - $y).abs() < $precision
    };
    ($x:expr, $y:expr, ?vec2) => {
        ($x - $y).abs().x < 0.001 && ($x - $y).abs().y < 0.001
    };
    ($x:expr, $y:expr, ?vec2, ?precision = $precision:expr) => {
        ($x - $y).abs().x < $precision && ($x - $y).abs().y < $precision
    };
    ($x:expr, $y:expr, ?vec3) => {
        ($x - $y).abs().x < 0.001 && ($x - $y).abs().y < 0.001 && ($x - $y).abs().z < 0.001
    };
    ($x:expr, $y:expr, ?vec3, ?precision = $precision:expr) => {
        ($x - $y).abs().x < $precision && ($x - $y).abs().y < $precision && ($x - $y).abs().z < $precision
    };
    ($x:expr, $y:expr, ?radian) => {
        ($x - $y).abs() < $crate::primitives::Radian(0.001)
    };
    ($x:expr, $y:expr, ?radian, ?precision = $precision:expr) => {
        ($x - $y).abs() < $crate::primitives::Radian($precision)
    }
}

/// gets the rotation in radians according to `source` and `destination`
///
/// starts from the X axis of source(right), **counter clock-wise**
/// 2D only
pub fn get_rotate_radian(source: Vec2, destination: Vec2) -> f32 {
    let x_diff = destination.x - source.x;
    let y_diff = destination.y - source.y;

    atan2(y_diff, x_diff)
}

/// centre point at middle of window
#[cfg(feature = "client")]
pub(crate) fn get_cursor_pos(
    window: &Window,
    camera: (&Camera, &GlobalTransform),
) -> Option<Vec2> {
    let (camera, camera_transform) = camera;
    window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.xy())
}

/// calculates Vec3 to add to `Transform.translation` from the rotation and speed
/// 
/// doesn't do anything with the Z-axis (why did i make that mistake....)
pub fn move_with_rotation(rotation: Radian, speed: Speed) -> Vec3 {
    (rotation.to_vec() * speed.get_raw()).extend(0.0)
}


/// gets a approximately round area of tiles around a point
/// # Unexpected behavior
/// the `radius` will be rounded, therefore only reeturning integer points
#[inline]
pub fn tiles_around_point(position: Vec2, radius: f32) -> Vec<Vec2> {
    let radius = radius.round() as i32;
    let mut ret = vec![];

    for r in -radius..radius {
        for r2 in -radius..radius {
            let tile = vec2(r as f32, r2 as f32) + position;
            if tile.distance(position) <= radius as f32 {
                ret.push(tile);
            }
        }
    }

    ret
}

const MUL_RANGE: Range<f32> = 0.8..1.2;
static X_MULTIPLIER: LazyLock<f32> = LazyLock::new(|| rand::random_range(MUL_RANGE));
static Y_MULTIPLIER: LazyLock<f32> = LazyLock::new(|| rand::random_range(MUL_RANGE));

/// similar to [`tiles_around_point`] but returns a square with radius randomnized
#[inline]
pub fn avaliable_cords(position: Vec2, radius: f32) -> (RangeInclusive<f32>, RangeInclusive<f32>) {
    let x = {
        let adjusted_radius = radius * *X_MULTIPLIER;
        let left = position.x - adjusted_radius;
        let right = position.x + adjusted_radius;
        left..=right
    };
    let y = {
        let adjusted_radius = radius * *Y_MULTIPLIER;
        let down = position.y - adjusted_radius;
        let up = position.y + adjusted_radius;
        down..=up
    };
    (x, y)
}

/// returns the (radius, darkness (0..1)) to be passed into shaders
///
/// the closer to the surface(0.0), the bigger the radius, smaller the darkness and vice versa
///
/// note that we're returning the maximum darkness if calculated value exceeds instead of calculating the darkness according
/// to the range between 0 and max_darkness
pub fn calculate_diving_overlay(
    altitude: ZIndex,
    ocean_floor: ZIndex,
    min_radius: f32,
    max_radius: f32,
    max_darkness: f32,
) -> (f32, f32) {
    if *altitude > 0.0 {
        return (max_radius, 0.0); // consider panicking?
    }

    assert!(*ocean_floor < 0.0);
    assert!(altitude >= ocean_floor);
    assert!(max_radius > min_radius);

    let diff = (ocean_floor - altitude).abs();
    let radius = diff / ocean_floor.abs() * (max_radius - min_radius) + min_radius;
    let darkness = 1.0 - (diff / ocean_floor.abs());

    if darkness > max_darkness {
        (radius, max_darkness)
    } else {
        (radius, darkness)
    }
}

#[inline]
pub fn point_in_square(point: Vec2, square_len: f32, square_center: Vec2) -> bool {
    let square = Mk48Rect::new(square_center, WidthHeight::splat(square_len));

    square.contains(point)
}

/// calculates a float from the given `current` and respective range (`minimum_source..=unit_1`).
/// #### Note
/// if `current` is bigger than `unit_1`, `maximum_value` will be returned.
///
/// if `current` is smaller than provided `minimum_source`, 0 will be returned.
/// ### Panics
/// if provided `minimum_source` is bigger than `unit_1`
pub fn calculate_from_proportion(
    current: f32,
    unit_1: f32,
    maximum_value: f32,
    minimum_source: f32,
) -> f32 {
    assert!(minimum_source <= unit_1);

    if current <= minimum_source {
        return 0.0;
    }
    if current >= unit_1 {
        return maximum_value;
    }
    let proportion = (current - minimum_source) / (unit_1 - minimum_source);

    maximum_value * proportion
}

/// calculates the circle HUD by adding 7/10 of `length` to `length`
pub fn add_circle_hud(length: f32) -> f32 {
    length * 0.7 + length
}

pub fn input_not_pressed<T>(input: T) -> impl Fn(Res<ButtonInput<T>>) -> bool + Clone
where
    T: Clone + Eq + std::hash::Hash + Send + Sync + 'static
{
    move |buttons| !buttons.pressed(input.clone())
}

/// known Euclidean coordinates, known angle to be rotated, calculates the
/// correct coordinates after rotation
pub(crate) fn rotate_vec2(source: Vec2, Radian(angle): Radian) -> Vec2 {
    vec2(
        source.x * angle.cos() - source.y * angle.sin(),
        source.y * angle.cos() + source.x * angle.sin(),
    )
}

/// adds the specified systems to the [`Update`] schedule in the app
#[macro_export]
macro_rules! add_dbg_app {
    ( $app:expr, $( $system:expr ),+ ) => {
        #[cfg(debug_assertions)]
        $app.add_systems(::bevy::app::Update, $(
            $system
        )+);
    };
}

pub trait VecDequeStartsWith {
    type Inside;
    fn starts_with(&self, needle: &[Self::Inside]) -> bool;
}

impl<T: PartialEq> VecDequeStartsWith for VecDeque<T> {
    type Inside = T;
    fn starts_with(&self, needle: &[Self::Inside]) -> bool {
        let (front, back) = self.as_slices();

        if needle.len() <= front.len() {
            front.starts_with(needle)
        } else {
            let (front_needle, back_needle) = needle.split_at(front.len());

            front == front_needle && back.starts_with(back_needle)
        }
    }
}
/// prints number of a entity with specified query filter passed in to console
/// filter defaults to [`With`]
/// ## Example
///
/// ```ignore
/// print_num!(&mut app, ActionState<Move>, InputMarker<Move>);
/// // expands to:
/// let system =  |query:Query<(), (With<ActionState<Move>>, With<InputMarker<Move>>)>| {
///     let len = query.iter().len();
///     info!("{} entities of {}", len, stringify!((ActionState<Move>, InputMarker<Move>)));
/// };
/// app.add_systems(Update, system);
/// ```
#[macro_export]
macro_rules! print_num {
    ($app:expr, $($filter:ty),*) => {
        let system = |query: Query<(), ( $(
            With<$filter>
        ),* ) >| {
            let len = query.iter().len();

            let mut filter_str = String::new();
            filter_str.push('(');
            $(
                filter_str.push_str(stringify!($filter));
                filter_str.push_str(", ");
            )*
            filter_str.push(')');
            info!("{} entities of {}", len, filter_str);
        };

        $app.add_systems(Update, system);
    };
}

#[macro_export]
macro_rules! debug_component {
    ($component:ty, $($filter:ty)?, $($condition:expr)?) => {
        |q: ::bevy::prelude::Query<&$component, $( $filter )?>| {
            let s = stringify!($component);
            for c in q {
                $( if $condition(&c) { 
                    ::bevy::log::info!("{}: {:?}", s, c);
                }
                continue;
                )?
                
                ::bevy::log::info!("{}: {:?}", s, c);
            }
        }
    };
}

#[macro_export]
macro_rules! hashmap {
    () => {
        ::std::collections::HashMap::new()
    };

    ($($key:expr => $value:expr),+ $(,)?) => {
        ::std::collections::HashMap::from([ $(($key, $value)),* ])
    };
}


// movements

/// extract or return
#[macro_export]
macro_rules! extract {
    ($in:expr, ?Option) => {
        match $in {
            Some(x) => x,
            None => return,
        }
    };
    ($in:expr, ?Result) => {
        match $in {
            Ok(x) => x,
            Err(e) => {
                error!("Unwrapping on Err({:?})", e);
                return;
            }
        }
    };
}

pub const fn ip_addr(hostname: Ipv4Addr, port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(hostname), port)
}

/// webtransport/websocket certificate, currently not used, using plain websockets
#[cfg(feature = "server")]
#[cfg(false)]  // not using websockets
pub fn from_pem_file(
    cert_path: impl AsRef<std::path::Path>,
    key_path: impl AsRef<std::path::Path>,
) -> Identity {
    use std::fs;

    let cert_chain_bytes = fs::read(cert_path).unwrap();
    let key_bytes = fs::read(key_path).unwrap();

    let mut cert_reader = std::io::Cursor::new(cert_chain_bytes);
    let certs = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let mut key_reader = std::io::Cursor::new(key_bytes);
    let key = rustls_pemfile::private_key(&mut key_reader)
        .unwrap()
        .unwrap();

    Identity::new(certs, key)
}


/// allows a generic [`into`](Into::into), has a blanket implementation
///
/// ```
///# use common::util::InputExt;
/// let x: u8 = 3;
/// let y = x.to::<u16>().to::<i32>().to::<i64>();
/// ```
pub trait InputExt
where
    Self: Sized,
{
    fn to<T>(self) -> T
    where
        T: From<Self>;
}

impl<T> InputExt for T {
    fn to<U>(self) -> U
    where
        U: From<Self>,
    {
        From::from(self)
    }
}

pub trait ResultExt {
    fn log(self) -> Self;
    fn log_err(self) -> Self;
}

impl<T: std::fmt::Debug, E: std::fmt::Debug> ResultExt for Result<T, E> {
    /// similar to [`Result::inspect`]
    /// 
    /// ### Output:
    /// INFO Val: {[`Ok`] variant's [`Debug`] impl}
    fn log(self) -> Self {
        if let Ok(ref t) = self {
            info!("Val: {:?}", t);
        }
        self
    }
    /// similar to [`Result::inspect_err`]
    /// 
    /// ### Output:
    /// ERROR Err: {[`Err`] variant's [`Debug`] impl}
    fn log_err(self) -> Self {
        if let Err(ref e) = self {
            error!("Err: {:?}", e);
        }
        self
    }
}

/// const alternative to [`px`]
pub const fn pixel(input: i32) -> Val {
    Val::Px(input as f32)
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, PartialOrd)]
pub struct OrderedHashMap<K, V> {
    vec: Vec<(K, V)>
}

impl<K, V> OrderedHashMap<K, V> {
    pub const fn new() -> Self {
        OrderedHashMap { vec: Vec::new() }
    }
    pub fn from_arr<const N: usize>(arr: [(K, V); N]) -> Self {
        OrderedHashMap { vec: Vec::from(arr) }
    }
    pub fn push(&mut self, key: K, value: V) {
        self.vec.push((key, value));
    }
    pub fn iter(&self) -> slice::Iter<'_, (K, V)> {
        self.vec.iter()
    }
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, (K, V)> {
        self.vec.iter_mut()
    }
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.vec.iter().map(|(k, _)| k)
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.vec.iter().map(|(_, v)| v)
    }
    pub const fn len(&self) -> usize {
        self.vec.len()
    }
    pub const fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }
}

impl<K, V> Default for OrderedHashMap<K, V> {
    fn default() -> Self {
        Self { vec: Vec::new() }
    }
}
impl<K: PartialEq, V> OrderedHashMap<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        self.vec.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }
    pub fn get_index(&self, key: &K) -> Option<usize> {
        self.vec.iter()
            .position(|(k, _)| k == key)
    }
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.vec.iter_mut()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }
}

impl<K, V> IntoIterator for OrderedHashMap<K, V> {
    type Item = (K, V);
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.into_iter()
    }
}
impl<'a, K, V> IntoIterator for &'a OrderedHashMap<K, V> {
    type Item = &'a (K, V);
    type IntoIter = slice::Iter<'a, (K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.iter()
    }
}
impl<'a, K, V> IntoIterator for &'a mut OrderedHashMap<K, V> {
    type Item = &'a mut(K, V);
    type IntoIter = slice::IterMut<'a, (K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.iter_mut()
    }
}
impl<K, V> Index<usize> for OrderedHashMap<K, V> {
    type Output = (K, V);
    fn index(&self, index: usize) -> &Self::Output {
        self.vec.index(index)
    }
}
/// Returns mutable access to specific item that appears in `children` first in `query`
/// 
/// Expand to
/// ```ignore
/// let entity = children.iter()
///     .find(|e| query.get(*e).is_ok())
///     .unwrap();
/// query.get_mut(entity)
/// ```
/// 
/// used to solve rust not passing this:
/// ```ignore
/// children.iter().find_map(|e| query.get_mut(e).ok())
/// ```
#[macro_export]
macro_rules! get_mut {
    ($children:expr, $query:expr) => {
        {
            let entity = $children.iter()
                .find(|e| $query.get(*e).is_ok()).expect("Couldn't find any children in query");

            $query.get_mut(entity).ok()
        }
    };
}

/// impl of `itertools::zip_longest`
/// 
/// return type iterates over Option<I1::Item> and Option<I2::Item>,
/// terminating when both hit None
#[allow(clippy::type_complexity)]
pub fn zip_longest<I1, I2>(
    first: I1,
    second: I2
) -> FromFn<impl FnMut() -> Option<(Option<I1::Item>, Option<I2::Item>)>>
where
    I1: IntoIterator, I2: IntoIterator
{
    let mut first = first.into_iter();
    let mut second = second.into_iter();
    std::iter::from_fn(move || {
        match (first.next(), second.next()) {
            (None, None) => None,
            (a, b) => Some((a, b))
        }
    })
}


pub trait InputEnabled<M> {
    /// equivalent of `.run_if(input_free)`
    /// 
    /// when a system/observer can only be run when the user is NOT interacting with UI
    fn normal_input(self) -> ScheduleConfigs<Box<dyn System<Out = (), In = ()> + 'static>>;
}

impl<T, M> InputEnabled<M> for T
where 
    T: IntoScheduleConfigs<Box<dyn System<Out = (), In = ()>>, M>
{
    fn normal_input(self) -> ScheduleConfigs<Box<dyn System<Out = (), In = ()> + 'static>> {
        self.run_if(input_free)
    }
}

#[derive(Resource, PartialEq)]
pub struct BlockInput(pub bool);

pub fn input_free(block_input: Res<BlockInput>) -> bool {
    !block_input.0
}

#[macro_export]
macro_rules! log_on_add {
    (<$target:ty>) => {
        |trigger: On<Add, $target>, query: Query<&$target>| {
            let Ok(comp) = query.get(trigger.entity) else {
                error!("{} was added but not found", stringify!($target));
                return;
            };
            debug!("{} was added: {:?}", stringify!($target), comp);
        } 
    };
}

pub trait FloatExt {
    /// discard numbers after floating point
    /// ```
    /// # use common::util::FloatExt;
    /// let f = 3.2f32;
    /// let g = 3.5f32;
    /// let h = 3.9f32;
    /// let i = 3.0f32;
    /// 
    /// assert_eq!(f.preserve_int(), 3.);
    /// assert_eq!(g.preserve_int(), 3.);
    /// assert_eq!(h.preserve_int(), 3.);
    /// assert_eq!(i.preserve_int(), 3.);
    /// ```
    fn preserve_int(self) -> Self;
}
impl FloatExt for f32 {
    fn preserve_int(self) -> Self {
        let rounded = self.round();
        if (self - rounded) < 0.0 {
            if self.is_sign_positive() {
                rounded - 1.0
            } else {
                rounded + 1.0
            }
        } else {
            rounded
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::primitives::WrapZIndex;
    use super::*;
    #[test]
    fn test_get_rotate_radians() {
        let source = vec2(10.0, 3.0);
        let destination = vec2(10.0, 5.0);

        assert_eq!(get_rotate_radian(source, destination).to_degrees(), 90.0);
    }
    #[test]
    fn test_move_with_rotation() {
        let rotation = Radian::from_deg(90.0);
        assert_eq!(move_with_rotation(rotation, Speed::from_raw(2.0)).y, 2.0);
    }
    #[test]
    fn test_add_circle_hud() {
        assert_eq!(add_circle_hud(10.0), 17.0);
    }
    #[test]
    fn test_calculate_from_proportion() {
        let source = 7.5;
        let minimum = 5.0;
        let unit_1 = 10.0;

        let maximum = 100.0;

        let result = calculate_from_proportion(source, unit_1, maximum, minimum);
        assert_eq!(result, 50.0);
    }
    #[test]
    fn test_div_overlay() {
        let target = calculate_diving_overlay(-0.4.wrap_z(), -2.0.wrap_z(), 30.0, 50.0, 0.4);

        assert!(eq!(target.1, 0.2));
        assert_eq!(target.0, 46.0);
    }
}
