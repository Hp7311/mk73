//! UI origin at top left, World origin at middle of screen
//! remember to add Camera2d and identifier struct on StartUp

use std::f32::consts::PI;

use bevy::{math::ops::atan2, prelude::*, window::PrimaryWindow};

const YASEN_MAX_TURN_DEGREE: f32 = 2.0;
const YASEN_RAW_SIZE: f32 = 1024.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(YasenPlugin)
        .run();
}

struct YasenPlugin;

impl Plugin for YasenPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::linear_rgb(0.0, 0.4, 0.8)))
            .add_systems(Startup, startup)
            .add_systems(Update, update_yasen);
    }
}
#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Yasen {
    /// maximum angle in radians that you can turn per frame
    /// ### Warning
    /// keep the value small
    max_turn_radian: f32,
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, MainCamera));  // ::default() ?
    commands.spawn((
        Sprite::from_image(
            asset_server.load("yasen.png")
        ),
        Transform::from_scale(Vec3::splat(0.3))
            .with_translation(Vec3 { x: 100.0, y: 0.0, z: 0.0 })
            .with_rotation(Quat::from_rotation_z(PI / (180.0 / 90.0))),
        Yasen {
            max_turn_radian: YASEN_MAX_TURN_DEGREE.to_radians(),
        }
    ));
}


// use Quat.to_euler!!!
fn update_yasen(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut yasen_transform: Query<(&mut Transform, &Yasen)>
) {
    if let Some(cursor_pos) = get_cursor_pos(window, camera) && buttons.pressed(MouseButton::Left) {
        for (mut transform, yasen) in yasen_transform.iter_mut() {

            let raw_moved = get_move_radian(cursor_pos, transform.translation.xy());
            let (_, _, current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

            let moved = {
                let mut raw_moved = (raw_moved.to_degrees() - current_rotation.to_degrees()).to_radians();
                if raw_moved > PI {
                    raw_moved -= 2.0 * PI;
                } else if raw_moved < -PI {
                    raw_moved += 2.0 * PI;
                }
                raw_moved
            };

            if moved.abs() > yasen.max_turn_radian {
                println!("Rotated too much, fallback...");
                if moved > 0.0 {
                    transform.rotate_local_z(yasen.max_turn_radian);
                } else if moved < 0.0 {
                    transform.rotate_local_z(-yasen.max_turn_radian);
                }
            } else {
                transform.rotate_local_z(moved);
            }
            println!("Moved from last time: {}", moved.to_degrees());
        }
    }
}

/// gets the rotation in radians according to `source` and `destination`
/// 
/// starts from the X axis of source(right), **counter clock-wise**
/// 2D only
fn get_move_radian(source: Vec2, destination: Vec2) -> f32 {
    let x_diff = source.x - destination.x;
    let y_diff = source.y - destination.y;

    atan2(y_diff, x_diff)
}


fn get_cursor_pos(window: Single<&Window, With<PrimaryWindow>>, camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>) -> Option<Vec2> {
    let (camera, camera_transform) = *camera;
    window.cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_move_degrees() {
        let source = vec2(10.0, 3.0);
        let destination = vec2(10.0, 5.0);

        assert_eq!(get_move_radian(source, destination).to_degrees(), -90.0);
    }
}