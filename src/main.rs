//! UI origin at top left, World origin at middle of screen
//! remember to add Camera2d and identifier struct on StartUp

use std::f32::consts::PI;

use bevy::{prelude::*, window::PrimaryWindow};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::linear_rgb(0.0, 0.4, 0.8)))
        .add_systems(Startup, startup)
        .add_systems(Update, update_yasen)
        .run();
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Yasen;

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, MainCamera));  // ::default() ?
    commands.spawn((
        Sprite::from_image(
            asset_server.load("yasen.png")
        ),
        Transform::from_scale(Vec3::splat(0.3))
            .with_translation(Vec3 { x: 100.0, y: 0.0, z: 0.0 })
            .with_rotation(Quat::from_rotation_z(PI / (180.0 / 90.0))),
        Yasen
    ));
}

// mindful of initial 180/90 rotation, originally ->
fn update_yasen(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut yasen_transform: Query<&mut Transform, With<Yasen>>
) {
    if buttons.just_pressed(MouseButton::Left) || buttons.pressed(MouseButton::Left) {
        let cursor_pos = get_cursor_pos(window, camera).unwrap();
        for mut transform in yasen_transform.iter_mut() {
            transform.rotation = get_move_degree(cursor_pos, transform.translation.xy());
        }
    }
}

/// gets the rotation according to mouse position
/// ## Important
/// assumes sprite facing right
fn get_move_degree(mouse_pos: Vec2, entity_pos: Vec2) -> Quat {
    let diff = mouse_pos - entity_pos;

    Quat::from_rotation_z(diff.y.atan2(diff.x))
}

fn get_cursor_pos(window: Single<&Window, With<PrimaryWindow>>, camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>) -> Option<Vec2> {
    let (camera, camera_transform) = *camera;
    window.cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
}