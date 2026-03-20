use bevy::{camera_controller::pan_camera::PanCameraPlugin, prelude::*};
use mk73::{BoatPlugin, OilRigPlugin, ShadersPlugin, WeaponPlugin, WorldPlugin};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::linear_rgb(0.0, 0.1, 0.6)))
        .add_systems(Startup, mk73::setup)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        canvas: Some("#bevy_canvas".to_owned()),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..Default::default()
                }),
        )
        .add_plugins(PanCameraPlugin)
        .add_plugins((
            ShadersPlugin,
            WorldPlugin,
            BoatPlugin,
            OilRigPlugin,
            WeaponPlugin,
        ))
        .run();
}
