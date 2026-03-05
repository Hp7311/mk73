use bevy::{camera_controller::pan_camera::PanCameraPlugin, prelude::*};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::linear_rgb(0.0, 0.1, 0.6)))
        .add_systems(Startup, mk73::setup)
        .add_plugins(DefaultPlugins)
        .add_plugins(PanCameraPlugin)
        .add_plugins(mk73::WorldPlugin)
        .add_plugins(mk73::BoatPlugin)
        .add_plugins(mk73::OilRigPlugin)
        .run();
}
