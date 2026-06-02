use std::collections::HashMap;

use bevy::prelude::*;
use common::{primitives::FetchSprite, util::InputExt};
use serde::Deserialize;

pub(crate) struct AssetPreloadPlugin;

impl Plugin for AssetPreloadPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(FontMap(HashMap::new()))

            .add_systems(Startup, init_spritesheet)
            .add_systems(Startup, init_font);
    }
}

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
fn init_spritesheet(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut textures: ResMut<Assets<TextureAtlasLayout>>
) {
    let sprite = asset_server.load("spritesheet.webp");

    let map = SpriteMap::new(sprite, &mut textures);
    commands.insert_resource(map);
    info!("Finished loading spritesheet")
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

#[derive(Debug, Resource)]
pub struct SpriteMap {
    image: Handle<Image>,
    /// in sync with [`atlas::textures`](TextureAtlasLayout::textures)
    names: Vec<String>,
    atlas: Handle<TextureAtlasLayout>
}

/// returns None if not found
impl SpriteMap {
    /// should be only called once
    pub fn new(image: Handle<Image>, textures: &mut Assets<TextureAtlasLayout>) -> Self {
        let sheet = SpriteSheet::new();
        let (names, atlas) = sheet.to_texture_atlas_names();
        let atlas = textures.add(atlas);
        
        let names = names.into_iter().map(ToOwned::to_owned).collect();

        Self {
            image,
            names,
            atlas
        }
    }
    /// e.g. the default `name` for [`Boat::Yasen`](common::Boat::Yasen) is its identifier "Yasen"
    pub fn get(&self, name: impl FetchSprite) -> Option<TextureAtlas> {
        let index = self.names.iter().position(|n| n == name.fetch_sprite_str().as_ref())?;

        Some(TextureAtlas {
            index, 
            layout: self.atlas.clone()
        })
    }
    pub fn get_index(&self, name: impl FetchSprite) -> Option<usize> {
        self.names.iter().position(|n| n == name.fetch_sprite_str().as_ref())
    }
    /// sets texture atlas to given name, returns None if not found
    #[allow(dead_code)]
    pub fn set_to(&self, name: impl FetchSprite, atlas: &mut TextureAtlas) -> Option<()> {
        atlas.index = self.get_index(name)?;

        Some(())
    }
    /// hide .clone s
    pub fn image(&self) -> Handle<Image> {
        self.image.clone()
    }
}

#[derive(Debug, Deserialize)]
pub struct SpriteSheet {
    pub frames: HashMap<String, SheetCell>,
    pub meta: Meta
}

impl SpriteSheet {
    pub fn new() -> Self {
        let json = include_str!("../assets/spritesheet.json");

        serde_json::from_str::<SpriteSheet>(json).unwrap()
    }
    /// - `path` relative to crate root (Cargo.toml)
    #[allow(unused)]
    pub fn from_file(path: &str) -> Self {
        let json = std::fs::read_to_string(path).unwrap();

        serde_json::from_str(&json).unwrap()
    }
    /// returns list of sprite names and texutre atlas (ordered, ret.0[0] == ret.1.textures.get)
    pub fn to_texture_atlas_names(&self) -> (Vec<&str>, TextureAtlasLayout) {
        let mut names = vec![];
        let mut textures = vec![];

        for (name, cell) in self.frames.iter() {
            names.push(name.as_str());
            textures.push(cell.frame.to::<URect>());
        }

        (names, TextureAtlasLayout {
            size: self.meta.size.into(),
            textures
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct SheetCell {
    pub rotated: bool,
    pub trimmed: bool,
    pub frame: Rect4,
    pub sprite_source_size: Rect4,
    pub source_size: Rect2
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    /// whole sprite size
    pub size: Rect2
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Rect4 {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Rect2 {
    pub w: u32,
    pub h: u32
}

impl From<Rect2> for UVec2 {
    fn from(value: Rect2) -> Self {
        uvec2(value.w, value.h)
    }
}
impl From<Rect4> for URect {
    fn from(value: Rect4) -> Self {
        let min = uvec2(value.x, value.y);
        let max = uvec2(min.x + value.w, min.y + value.h);

        URect {
            min,
            max 
        }
    }
}