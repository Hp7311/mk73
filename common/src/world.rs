use bevy::prelude::*;

use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::{
    Boat,
    collision::{out_of_bound_no_rotation, out_of_bounds},
    primitives::{CustomTransform, Mk48Rect},
    protocol::OilRigTransform
};
#[cfg(feature = "server")]
use lightyear::prelude::{Connected, Disconnected, NetworkTarget};

#[cfg(feature = "client")]
use crate::{
    MainCamera,
    primitives::CursorPos,
    util::get_cursor_pos,
    shaders::WorldMaterial
};

#[allow(unused)]
const SPRITE_TINT: Color = Color::srgb(0.0, 0.65, 1.03);

/// - server: worldsize replicated, server should update worldsize on new client
/// - client: spawns map, spawns and updates cursorpos and shaders
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "server")]
        app.add_systems(Startup, spawn_worldsize)
            .add_observer(on_new_client)
            .add_observer(on_client_disconnected);

        #[cfg(feature = "client")]
        app.init_resource::<CursorPos>()
            // relies on replicated worldsize for determining size of sprite
            .add_observer(spawn_sprite)
            // not FixedUpdate due to small 誤差
            .add_systems(Update, update_cursor_pos)
            .add_systems(Update, update_sprite_size);
        #[cfg(feature = "client")]
        app.add_plugins(crate::shaders::ShaderPlugin);
    }
}

// TODO make this a resource when lightyear re-adds it
#[derive(Component, Debug, Copy, Clone, Deserialize, Serialize, PartialEq)]
pub struct WorldSize {
    current_expand: u32,
    player_num: u32,
    /// avoid performance penalty
    computed: Vec2
}

impl WorldSize {
    const WORLD_MIN: Vec2 = vec2(3000.0, 1500.0);
    const WORLD_EXPAND: Vec2 = Vec2::splat(500.0);

    /// 0 players
    fn new() -> Self {
        WorldSize {
            current_expand: get_multiplayer_by_player_num(0),
            player_num: 0,
            computed: get_map_size(0, Self::WORLD_MIN, Self::WORLD_EXPAND),
        }
    }
    pub fn player_num(&self) -> u32 {
        self.player_num
    }
    pub fn get_size(&self) -> Vec2 {
        self.computed
    }
    /// assumes center is (0, 0)
    pub fn to_rect(&self) -> Rect {
        Rect::from_center_size(Vec2::ZERO, self.computed)
    }
}

impl Default for WorldSize {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "client")]
#[derive(Component, Debug, Copy, Clone)]
struct Background;

#[cfg(feature = "client")]
use client::*;
#[cfg(feature = "client")]
mod client {
use super::*;
use lightyear::prelude::Replicated;

pub fn spawn_sprite(
    trigger: On<Add, WorldSize>,
    world_size: Query<&WorldSize, With<Replicated>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<WorldMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>
) {
    if world_size.iter().len() != 1 {
        error!("Expected 1 worldsize");
        return;
    }
    let Ok(world_size) = world_size.get(trigger.entity) else { unreachable!("above handled") };

    commands.spawn((
        Transform {
            translation: Vec3 {
                z: -1.0,
                ..default()
            },
            ..default()
        },
        // TODO add graphics features to background. Right now a Sprite::from_color can replace it
        // e.g. wave effect around controlling player's ocean (will take a lot of time)
        Mesh2d(meshes.add(Rectangle::from_size(world_size.get_size()))),
        MeshMaterial2d(materials.add(WorldMaterial::from_srgb_u8(1, 14, 41))),
        Name::new("Background"),
        Background,
    ));
}


pub fn update_sprite_size(mut meshes: ResMut<Assets<Mesh>>, sprite: Single<&Mesh2d, With<Background>>, world_size: Single<&WorldSize, Changed<WorldSize>>) {
    // sprite.custom_size = Some(world_size.get_size());
    if let Some(mesh) = meshes.get_mut(*sprite) {
        *mesh = Rectangle::from_size(world_size.get_size()).into();
    }
}

pub fn update_cursor_pos(
    mut cursor_pos: ResMut<CursorPos>,
    // mut move_event: MessageReader<CursorMoved>
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>
) {
    if let Some(pos) = get_cursor_pos(&window, *camera)
        && pos != cursor_pos.0
    {
        cursor_pos.0 = pos;
    }
    // for moved in move_event.read() {
    //     cursor_pos.0 = moved.position;
    // }
}

}


#[cfg(feature = "server")]
use server::*;
#[cfg(feature = "server")]
mod server {
use crate::primitives::Size;

use super::*;
use lightyear::prelude::Replicate;

#[derive(Debug)]
struct ZeroPlayerLeft;

impl WorldSize {
fn add_player(&mut self) {
    self.player_num += 1;
    self.current_expand = get_multiplayer_by_player_num(self.player_num);
    self.computed = get_map_size(self.player_num, Self::WORLD_MIN, Self::WORLD_EXPAND)
}
fn remove_player(&mut self) -> Result<(), ZeroPlayerLeft> {
    if self.player_num == 0 {
        return Err(ZeroPlayerLeft)
    }
    self.player_num -= 1;
    self.current_expand = get_multiplayer_by_player_num(self.player_num);
    self.computed = get_map_size(self.player_num, Self::WORLD_MIN, Self::WORLD_EXPAND);

    Ok(())
}
}

pub fn spawn_worldsize(mut commands: Commands) {
    commands.spawn((
        WorldSize::new(),
        Replicate::to_clients(NetworkTarget::All)
    ));
}

pub fn on_new_client(
    _trigger: On<Add, Connected>,
    mut world_size: Query<&mut WorldSize>
) {
    let Ok(mut world_size) = world_size.single_mut().inspect_err(|e| error!("expected only one worldsize: {e:?}")) else { return; };
    world_size.add_player();
}
/// shrink world
/// 
/// despawn oil rigs that are outofbound
/// 
/// clamp players back within the borders
pub fn on_client_disconnected(
    _trigger: On<Add, Disconnected>,
    mut world_size: Query<&mut WorldSize>,
    customs: Query<(&mut CustomTransform, &Boat, Entity)>,
    rigs: Query<(&OilRigTransform, Entity)>,
    mut commands: Commands
) {
    let Ok(mut world_size) = world_size.single_mut().inspect_err(|e| error!("expected only one worldsize: {e:?}")) else { return; };
    if world_size.remove_player().is_err() {
        warn!("Trying to remove player when no more players left, potentially from initiating non-authorized WS");
    }

    // push players back
    for (mut custom, boat, id) in customs {
        if commands.get_spawned_entity(id).is_err() {
            continue;
        }
        if out_of_bounds(&world_size, Mk48Rect::new(custom.position.0, boat.render_size()), custom.rotation) {
            let [min, max] = Mk48Rect::new(Vec2::ZERO, world_size.get_size()).clamp_corners();
            custom.position = custom.position.clamp_with_padding(min, max, boat.render_size().max_element());
        }
    }
    for (transform, entity) in rigs {
        if out_of_bound_no_rotation(&world_size, Mk48Rect::new(transform.position, OilRigTransform::custom_size())) {
            commands.get_entity(entity).unwrap()
                .despawn();
        }
    }
}

}


/// gets the size of the World from the `minimum_size` and provided expand by per multiple
fn get_map_size(player_num: u32, minimum_size: Vec2, expand_per_multiple: Vec2) -> Vec2 {
    minimum_size + expand_per_multiple * get_multiplayer_by_player_num(player_num) as f32
}

fn get_multiplayer_by_player_num(player_num: u32) -> u32 {
    match player_num {
        0..20 => 1,
        20..50 => 2,
        50..130 => 3,
        130..200 => 4,
        200..300 => 5,
        300..400 => 6,
        _ => 7,
    }
}
