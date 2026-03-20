//! utility functions independent to game

// remember high test coverage
use bevy::{math::ops::atan2, prelude::*, window::PrimaryWindow};

use crate::{
    MainCamera,
    primitives::{DecimalPoint, MkRect, WidthHeight},
};

/// the equivalent of `==` only with a specified precision
pub(crate) fn eq(x: f32, y: f32, precision: DecimalPoint) -> bool {
    (x - y).abs() <= precision.to_f32()
}

/// the equivalent of `==` only with a specified precision
pub(crate) fn vec2_eq(x: Vec2, y: Vec2, precision: DecimalPoint) -> bool {
    let subtracted = (x - y).abs();
    subtracted.x <= precision.to_f32() && subtracted.y <= precision.to_f32()
}

/// gets the rotation in radians according to `source` and `destination`
///
/// starts from the X axis of source(right), **counter clock-wise**
/// 2D only
pub(crate) fn get_rotate_radian(source: Vec2, destination: Vec2) -> f32 {
    let x_diff = destination.x - source.x;
    let y_diff = destination.y - source.y;

    atan2(y_diff, x_diff)
}

/// calculates Vec3 to add to `Transform.translation` from the rotation and speed
/// ### Note
/// assumes 2D
pub(crate) fn move_with_rotation(rotation: Quat, speed: f32) -> Vec3 {
    let (.., move_angle) = rotation.to_euler(EulerRot::XYZ);

    (vec2(move_angle.cos(), move_angle.sin()) * speed).extend(0.0)
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

/// gets a approximately round area of tiles around a point
/// # Unexpected behavior
/// the `radius` will be rounded
pub(crate) fn tiles_around_point(position: Vec2, radius: f32) -> Vec<Vec2> {
    let radius_rg = radius.round() as i32;
    let mut ret = vec![];

    for r in -radius_rg..radius_rg {
        for r2 in -radius_rg..radius_rg {
            let tile = vec2(r as f32, r2 as f32) + position;
            if tile.distance(position) <= radius {
                ret.push(tile);
            }
        }
    }

    ret
}

/// returns the (radius, darkness (0..1)) to be passed into shaders
///
/// the closer to the surface(0.0), the bigger the radius, smaller the darkness and vice versa
///
/// note that we're returning the maximum darkness if calculated value exceeds instead of calculating the darkness according
/// to the range between 0 and max_darkness
pub(crate) fn calculate_diving_overlay(
    altitude: f32,
    ocean_floor: f32,
    min_radius: f32,
    max_radius: f32,
    max_darkness: f32,
) -> (f32, f32) {
    if altitude > 0.0 {
        return (max_radius, 0.0); // consider panicking?
    }

    assert!(ocean_floor < 0.0);
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


pub(crate) fn point_in_square(point: Vec2, square_len: f32, square_center: Vec2) -> bool {
    let square = MkRect {
        center: square_center,
        dimensions: WidthHeight::splat(square_len),
    };

    square.contains(point)
}

/// calculates a float from the given `current` and respective range (`minimum_source..=unit_1`).
/// #### Note
/// if `current` is bigger than `unit_1`, `maximum_value` will be returned.
///
/// if `current` is smaller than provided `minimum_source`, 0 will be returned.
/// ### Panics
/// if provided `minimum_source` is bigger than `unit_1`
pub(crate) fn calculate_from_proportion(
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
pub(crate) fn add_circle_hud(length: f32) -> f32 {
    length * 0.7 + length
}

/// rotates a local point by angle
pub(crate) fn rotate_vec2(source: Vec2, angle: Quat) -> Vec2 {
    let angle = angle.to_euler(EulerRot::XYZ).2;

    vec2(
        source.x * angle.cos() - source.y * angle.sin(),
        source.y * angle.cos() + source.x * angle.sin(),
    )
}

/// create a large bounding box that is guaranteed to contain the specified rectangle no matter the rotation
pub(crate) fn large_bounding_box(center: Vec2, dimensions: WidthHeight) -> MkRect {
    MkRect {
        center,
        dimensions: WidthHeight::splat(dimensions.max_side() * 1.5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_rotate_radians() {
        let source = vec2(10.0, 3.0);
        let destination = vec2(10.0, 5.0);

        assert_eq!(get_rotate_radian(source, destination).to_degrees(), 90.0);
    }
    #[test]
    fn test_move_with_rotation() {
        let rotation = Quat::from_rotation_z(90.0_f32.to_radians());
        assert_eq!(move_with_rotation(rotation, 2.0).y, 2.0);
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
        let target = calculate_diving_overlay(-0.4, -2.0, 30.0, 50.0, 0.4);

        assert!(eq(target.1, 0.2, DecimalPoint::Three));
        assert_eq!(target.0, 46.0);
    }
}
