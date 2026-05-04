//! utility functions independent to game

// remember high test coverage
use bevy::{math::ops::atan2, prelude::*, window::PrimaryWindow};
use lightyear::websocket::server::Identity;
use crate::MainCamera;
use crate::primitives::{Radian, Speed};
use crate::primitives::{Mk48Rect, WidthHeight};


pub fn in_states_2<T: States>(first: T, second: T)  -> impl Fn(Res<State<T>>) -> bool {
    move |state| *state.get() == first || *state.get() == second
}

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
pub(crate) fn get_cursor_pos(
    window: &Single<&Window, With<PrimaryWindow>>,
    camera: &Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) -> Option<Vec2> {
    let (camera, camera_transform) = **camera;
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

use crate::primitives::ZIndex;
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
        $app.add_systems(Update, $(
            $system
        )+);
    };
}

/// prints number of a entity with specified query filter passed in to console
/// filter defaults to [`With`]
/// ## Example
///
/// ```norun
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

// movements

/// extract or return
#[macro_export]
macro_rules! extract {
    ($in:expr, Option) => {
        match $in {
            Some(x) => x,
            None => return,
        }
    };
    ($in:expr, Result) => {
        match $in {
            Ok(x) => x,
            Err(e) => {
                error!("Unwrapping on Err({:?})", e);
                return;
            }
        }
    };
}

/// webtransport/websocket certificate, currently not used, using plain websockets
#[cfg_attr(debug_assertions, allow(dead_code))]
fn from_pem_file(
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
