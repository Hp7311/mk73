use std::collections::HashMap;

use bevy::prelude::*;
use common::primitives::FileName;

pub(crate) struct AssetPreloadPlugin;

impl Plugin for AssetPreloadPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpriteMap(HashMap::new()))
            .insert_resource(FontMap(HashMap::new()))

            .add_systems(Startup, init_sprites)
            .add_systems(Startup, init_font);
    }
}

/// list of paths to load sprites from
/// 
/// putting on top = loads first
const SPRITE_PATHS: &[&str] = &[
    "yasen.png",
    "momi.png",
    "zubr.png",
    "Set65.png",
    "oil_platform.png",
    "coin.png",
    "scrap.png",
    "barrel.png",
    // "waves.png",
    // "textures.png",
];
const FONT_PATHS: &[&str] = &[
    "regular.otf"
];

/// Stores assets by their path,
/// 
/// 
/// Asset system:
/// - [`Assets<Image>`]
///     - `Vec` of loaded images
///     
/// - [`Handle<Image>`]
///     - contains id of an image
///     - taken by `Sprite`
///     - [`AssetServer::load`] automatically adds loaded image to `Assets` and returns a `Handle`
#[derive(Debug, Resource)]
pub struct SpriteMap(HashMap<&'static str, Handle<Image>>);

fn init_sprites(mut map: ResMut<SpriteMap>, asset_server: Res<AssetServer>) {
    info!("Started loading sprites");
    for &path in SPRITE_PATHS {
        if map.0.insert(path, asset_server.load(path)).is_some() {
            warn!("Re-inserting image at {}", path);
        }
    }
    info!("Finished loading sprites");
}
#[derive(Debug, Resource)]
pub struct FontMap(HashMap<&'static str, Handle<Font>>);

fn init_font(mut map: ResMut<FontMap>, asset_server: Res<AssetServer>) {
    for &path in FONT_PATHS {
        if map.0.insert(path, asset_server.load(path)).is_some() {
            warn!("Re-inserting font at {}", path);
        }
    }
}

// consider merging 2 maps to 1 generic
#[allow(dead_code)]
impl SpriteMap {
    /// doesn't move out of internal [`HashMap`] therefore keeping the Asset even if the returned handle is droppeed
    pub fn get_long_lived(&self, path: FileName) -> Handle<Image> {
        self.0.get(path.0)
            .unwrap_or_else(|| panic!("file name incorrect: {}, implementation should gaurantee correctness", path.0))
            .clone()
    }
    // sprite will not be avaliable after this
    /// moves asset out herefore the Asset will be dropped if the returned handle is droppeed
    /// 
    /// ### Warning
    /// trying to access a [`Handle`] again would return None,
    /// 
    /// only use this method if you're absolutely sure you only need the Sprite once
    pub fn get(&mut self, path: FileName) -> Handle<Image> {
        self.0.remove(path.0)
            .unwrap_or_else(|| panic!("file name {} doesn't exist, 1: wrong name, 2: dropped", path.0))
            .clone()
    }
    pub fn id(&self, path: &str) -> Option<AssetId<Image>> {
        self.0.get(path).map(|s| s.id())
    }
}
#[allow(dead_code)]
impl FontMap {
    /// doesn't move out of internal [`HashMap`] therefore keeping the Asset even if the returned handle is droppeed
    pub fn get_long_lived(&self, path: &str) -> Option<Handle<Font>> {
        self.0.get(path).cloned()
    }
    /// moves asset out herefore the Asset will be dropped if the returned handle is droppeed
    /// 
    /// ### Warning
    /// trying to access a [`Handle`] again would return None,
    /// 
    /// only use this method if you're absolutely sure you only need the Sprite once
    pub fn get(&mut self, path: &str) -> Option<Handle<Font>> {
        self.0.remove(path)
    }
    pub fn id(&self, path: &str) -> Option<AssetId<Font>> {
        self.0.get(path).map(|s| s.id())
    }
}