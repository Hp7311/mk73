use mk73::ShipPlugin;
use bevy::prelude::{App, DefaultPlugins};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ShipPlugin)
        .run();
}