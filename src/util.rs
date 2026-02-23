//! utility functions independent to game

// remember high test coverage
use bevy::{math::ops::atan2, prelude::*, window::PrimaryWindow};

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
    let (_, _, move_angle) = rotation.to_euler(EulerRot::XYZ);

    (vec2(move_angle.cos(), move_angle.sin()) * speed)
        .extend(0.0)
}

/// centre point at middle of window
pub fn get_cursor_pos(window: Single<&Window, With<PrimaryWindow>>, camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>) -> Option<Vec2> {
    let (camera, camera_transform) = *camera;
    window.cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
}

/// calculates a float from the given `current` and respective range (`minimum_source..=unit_1`).
/// #### Note
/// if `current` is bigger than `unit_1`, `maximum_value` will be returned.
/// 
/// if `current` is smaller than provided `minimum_source`, 0 will be returned.
/// ### Panics
/// if provided `minimum_source` is bigger than `unit_1`
pub fn calculate_from_proportion(current: f32, unit_1: f32, maximum_value: f32, minimum_source: f32) -> f32 {
    assert!(minimum_source <= unit_1);

    if current <= minimum_source {
        return 0.0;
    }
    if current >= unit_1 {
        return maximum_value;
    }
    let proportion = (current - minimum_source) / (unit_1 - minimum_source);

    println!("Unit1: {unit_1}, minimum: {minimum_source}");
    println!("Proportion: {proportion}");
    maximum_value * proportion
}

/// calculates the circle HUD by adding 7/10 of `length` to `length`
pub fn add_circle_hud(length: f32) -> f32 {
    length * 0.7 + length
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
