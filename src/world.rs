use bevy::prelude::*;

use crate::primitives::WidthHeight;

const WORLD_MIN: Vec2 = vec2(2000.0, 1000.0);
const WORLD_EXPAND: f32 = 500.0;

const SPRITE_TINT: Color = Color::srgb(0.0, 0.65, 1.03);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub(crate) struct WorldSize(pub(crate) WidthHeight);

impl Default for WorldSize {
    fn default() -> Self {
        WorldSize(get_map_size(1, WORLD_MIN, WORLD_EXPAND).into())
    }
}

#[derive(Component, Debug, Copy, Clone)]
struct Background;


fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // in Sprites, translation is the center point of the Sprite
    let world_size = WorldSize::default().0;
    commands.spawn((
        Transform {
            translation: Vec3 {
                z: -1.0,
                ..default()
            },
            ..default()
        },
        Sprite {
            image: asset_server.load("waves.png"),
            color: SPRITE_TINT,
            custom_size: Some(world_size.to_vec2()),
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: 2.0,
            },
            ..default()
        },
        Background,
    ));
    
    commands.spawn(WorldSize(world_size));
}

/// gets the size of the World from the `minimum_size` and provided expand by per multiple
/// 
/// assumes that expand both axis
fn get_map_size(player_num: u32, minimum_size: Vec2, expand_per_multiple: f32) -> Vec2 {
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
