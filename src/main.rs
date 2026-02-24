use bevy::prelude::{App, DefaultPlugins};
use mk73::ShipPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ShipPlugin)
        .run();
}
