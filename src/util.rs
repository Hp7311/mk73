//! utility functions independent to game

// remember high test coverage
use bevy::{math::ops::atan2, prelude::*, window::PrimaryWindow};

use crate::{DEFAULT_SPRITE_SHRINK, MainCamera};

/// gets the rotation in radians according to `source` and `destination`
///
/// starts from the X axis of source(right), **counter clock-wise**
/// 2D only
pub(crate) fn get_rotate_radian(source: Vec2, destination: Vec2) -> f32 {
    let x_diff = source.x - destination.x;
    let y_diff = source.y - destination.y;

    atan2(y_diff, x_diff)
}

/// calculates Vec3 to add to `Transform.translation` from the rotation and speed
/// ### Note
/// assumes 2D
pub(crate) fn move_with_rotation(rotation: Quat, speed: f32, z_index: f32) -> Vec3 {
    let (.., move_angle) = rotation.to_euler(EulerRot::XYZ);

    (vec2(move_angle.cos(), move_angle.sin()) * speed).extend(z_index)
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
        .map(|ray| ray.origin.truncate())
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

pub(crate) fn point_in_square(point: Vec2, square_len: f32, square_center: Vec2) -> bool {
    let square = Rect::from_center_size(square_center, Vec2::splat(square_len));

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

pub(crate) fn rotate_vec2(source: Vec2, angle: Quat) -> Vec2 {
    let angle = angle.to_euler(EulerRot::XYZ).2;

    vec2(
        source.x * angle.cos() - source.y * angle.sin(),
        source.y * angle.cos() + source.x * angle.sin(),
    )
}

/// resize [`Sprite`]s by default constant
/// ### The sprite's `custom_size` attribute is modified, NOT the `transform`
#[deprecated]
pub(crate) fn resize_inner<T: Component>(
    mut sprites: Query<&mut Sprite, With<T>>,
    assets: &Res<Assets<Image>>,
) {
    for mut sprite in sprites.iter_mut() {
        let Some(image) = assets.get(&sprite.image) else {
            continue;
        };
        if sprite.custom_size.is_some() {
            continue;
        }

        sprite.custom_size = Some(vec2(
            image.width() as f32 * DEFAULT_SPRITE_SHRINK,
            image.height() as f32 * DEFAULT_SPRITE_SHRINK,
        ));
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
        assert_eq!(move_with_rotation(rotation, 2.0, 0.0).y, 2.0);
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
