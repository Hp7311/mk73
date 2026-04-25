use bevy::{prelude::*, window::PrimaryWindow};
use lightyear::prelude::{Connected, DeltaManager, Diffable, Disconnected, NetworkTarget, Replicate, Replicated, Server};
use serde::{Deserialize, Serialize};

use crate::{primitives::{CursorPos, WidthHeight}, util::get_cursor_pos, MainCamera};

const SPRITE_TINT: Color = Color::srgb(0.0, 0.65, 1.03);

/// - server: worldsize replicated, server should update worldsize on new client
/// - client: spawns map, spawns and updates cursorpos
pub struct WorldPlugin {
    pub is_server: bool
}

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        if self.is_server {
            app.add_observer(spawn_worldsize)
                .add_observer(on_new_client)
                .add_observer(on_client_disconnected);
        } else {
            app.init_resource::<CursorPos>()
                // relies on replicated worldsize for determining size of sprite
                .add_observer(spawn_sprite)
                // not FixedUpdate due to small 誤差
                .add_systems(Update, update_cursor_pos)
                .add_systems(Update, update_sprite_size);
        }
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

impl Diffable<u32> for WorldSize {
    fn base_value() -> Self {
        Self::new()
    }

    fn diff(&self, new: &Self) -> u32 {
        new.player_num
    }

    fn apply_diff(&mut self, delta: &u32) {
        self.player_num = *delta;

        self.computed = get_map_size(self.player_num, Self::WORLD_MIN, Self::WORLD_EXPAND);
        self.current_expand = get_multiplayer_by_player_num(self.player_num);
    }
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

    pub fn add_player(&mut self) {
        self.player_num += 1;
        self.computed += Self::WORLD_EXPAND * (get_multiplayer_by_player_num(self.player_num) - self.current_expand) as f32;
    }
    pub fn remove_player(&mut self) {
        assert_ne!(self.player_num, 0, "0 player left");
        self.player_num -= 1;
        self.computed -= Self::WORLD_EXPAND * (get_multiplayer_by_player_num(self.player_num) - self.current_expand) as f32;
    }
    pub fn player_num(&self) -> u32 {
        self.player_num
    }
    pub fn get_size(&self) -> Vec2 {
        self.computed
    }
    pub fn to_rect(&self, center: Vec2) -> Rect {
        Rect::from_center_size(center, self.computed)
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Background;

fn spawn_sprite(
    trigger: On<Add, WorldSize>,
    world_size: Query<&WorldSize, With<Replicated>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    if world_size.iter().len() != 1 {
        error!("Expected 1 worldsize");
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
        Sprite {
            image: asset_server.load("waves.png"),
            color: SPRITE_TINT,
            custom_size: Some(world_size.get_size()),
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: 2.0,
            },
            ..default()
        },
        Background,
    ));
}

fn update_sprite_size(mut sprite: Single<&mut Sprite, With<Background>>, world_size: Single<&WorldSize, Changed<WorldSize>>) {
    sprite.custom_size = Some(world_size.get_size());
}

fn update_cursor_pos(
    mut cursor_pos: ResMut<CursorPos>,
    // mut move_event: MessageReader<CursorMoved>
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>
) {
    if let Some(pos) = get_cursor_pos(&window, &camera)
        && pos != cursor_pos.0
    {
        cursor_pos.0 = pos;
    }
    // for moved in move_event.read() {
    //     cursor_pos.0 = moved.position;
    // }
}

// -- server ---


fn spawn_worldsize(server: On<Add, Server>, mut commands: Commands) {
    let world_size = WorldSize::new();

    commands.spawn((
        world_size,
        Replicate::to_clients(NetworkTarget::All)
    ));

    commands.get_entity(server.entity).unwrap()
        .insert(DeltaManager::default());
}

fn on_new_client(
    _trigger: On<Add, Connected>,
    mut world_size: Query<&mut WorldSize>
) {
    let Ok(mut world_size) = world_size.single_mut().inspect_err(|e| error!("expected only one worldsize: {e:?}")) else { return; };
    world_size.add_player();
    info!("Adding WorldSize for client");
}
fn on_client_disconnected(
    _trigger: On<Add, Disconnected>,
    mut world_size: Query<&mut WorldSize>
) {
    let Ok(mut world_size) = world_size.single_mut().inspect_err(|e| error!("expected only one worldsize: {e:?}")) else { return; };
    world_size.remove_player();
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
// TODO test this by setting smaller