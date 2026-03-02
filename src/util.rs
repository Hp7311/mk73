//! utility functions independent to game

use std::f32::consts::PI;

// remember high test coverage
use bevy::{math::ops::atan2, prelude::*, window::PrimaryWindow};

use crate::primitives::{RectIntersect, WidthHeight, WorldSize};

#[derive(Component)]
pub struct MainCamera;

/// gets the rotation in radians according to `source` and `destination`
///
/// starts from the X axis of source(right), **counter clock-wise**
/// 2D only
pub fn get_rotate_radian(source: Vec2, destination: Vec2) -> f32 {
    let x_diff = source.x - destination.x;
    let y_diff = source.y - destination.y;

    atan2(y_diff, x_diff)
}

/// calculates Vec3 to add to `Transform.translation` from the rotation and speed
/// ### Note
/// assumes 2D
pub fn move_with_rotation(rotation: Quat, speed: f32) -> Vec3 {
    let (.., move_angle) = rotation.to_euler(EulerRot::XYZ);

    (vec2(move_angle.cos(), move_angle.sin()) * speed)
        .extend(0.0)  // TODO assume Z index 0.0
}


/// centre point at middle of window
pub fn get_cursor_pos(
    window: &Single<&Window, With<PrimaryWindow>>,
    camera: &Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) -> Option<Vec2> {
    let (camera, camera_transform) = **camera;
    window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
}

/// check if a Sprite is out-of-bounds, returns true if out-of-bound
/// 
/// accounts for rotation by rotating each corner before checking bounds
pub fn out_of_bounds(
    bound: &WorldSize,
    sprite_size: WidthHeight,
    pos: Vec2,
    rotation: Quat,
) -> bool {
    let world_bound = Rect::new(
        -bound.0.width / 2.0,
        -bound.0.height / 2.0, 
        bound.0.width / 2.0,
        bound.0.height / 2.0
    );
    let corners = corners_rotated(pos, sprite_size, rotation);

    corners.iter().any(|corner| !world_bound.contains(*corner))
}

/// returns the global coordinates of a rotated Rectangle with given stats
pub fn corners_rotated(
    position: Vec2,
    dimensions: WidthHeight,
    rotation: Quat
) -> [Vec2; 4] {
    let half_size = vec2(dimensions.width / 2.0, dimensions.height / 2.0);
    [
        vec2(-half_size.x, -half_size.y),
        vec2(half_size.x, -half_size.y),
        vec2(half_size.x, half_size.y),
        vec2(-half_size.x, half_size.y)
    ]
    .map(|corner_local| rotate_vec2(corner_local, rotation) + position)
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

/// gets the size of the World from the `minimum_size` and provided expand by per multiple
/// 
/// assumes that expand both axis
pub fn get_map_size(player_num: u32, minimum_size: Vec2, expand_per_multiple: f32) -> Vec2 {
    let expand_per_multiple = Vec2::splat(expand_per_multiple);
    let multiplier = match player_num {
        0..20 => 1,
        20..50 => 2,
        50..130 => 3,
        130..200 => 4,
        200..300 => 5,
        300..400 => 6,
        _ => 7,
    };

    minimum_size + expand_per_multiple * (multiplier as f32)
}


/// rotates a point around (0, 0) for `angle`
pub fn rotate_vec2(source: Vec2, angle: Quat) -> Vec2 {
    let angle = angle.to_euler(EulerRot::XYZ).2;

    vec2(
        source.x * angle.cos() - source.y * angle.sin(),
        source.y * angle.cos() + source.x * angle.sin(),
    )
}

/// eliminates offset when turning over the <--- axis
pub trait TrimRadian {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_rotate_radians() {
        let source = vec2(10.0, 3.0);
        let destination = vec2(10.0, 5.0);

        assert_eq!(get_rotate_radian(source, destination).to_degrees(), -90.0);
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
}
