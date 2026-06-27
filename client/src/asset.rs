use std::collections::HashMap;

use bevy::prelude::*;
use common::{primitives::FetchSprite, util::InputExt};
use serde::Deserialize;

pub(crate) struct AssetPreloadPlugin;

impl Plugin for AssetPreloadPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, (init_spritesheet, init_sprite_ui_sheet));
    }
}


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
}

fn init_sprite_ui_sheet(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut textures: ResMut<Assets<TextureAtlasLayout>>
) {
    let sprite = asset_server.load("sprites_css.png");

    let map = SpriteUiMap::new(sprite, &mut textures);
    commands.insert_resource(map);
}

#[derive(Debug, Resource, Clone)]  // cloning only clones the Vec of names
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

/// currently, image is from Mk48, to be sustanible, better to generate ourselves // TODO
#[derive(Debug, Resource)]
pub struct SpriteUiMap {
    image: Handle<Image>,
    names: Vec<String>,
    atlas: Handle<TextureAtlasLayout>,
    /// stores the actual atlas without Handle (references)
    _atlas: TextureAtlasLayout
}

impl SpriteUiMap {
    fn new(image: Handle<Image>, textures: &mut Assets<TextureAtlasLayout>) -> Self {
        let sheet = SpriteUiSheet::new();
        let (names, atlas) = sheet.to_texture_atlas_names();
        let atlas_handle = textures.add(atlas.clone());
        
        let names = names.into_iter().map(ToOwned::to_owned).collect();

        Self {
            image,
            names,
            atlas: atlas_handle,
            _atlas: atlas
        }
    }
    pub fn get(&self, name: impl FetchSprite) -> Option<TextureAtlas> {
        let index = self.names.iter().position(|n| n == name.fetch_sprite_str().as_ref())?;

        Some(TextureAtlas {
            index, 
            layout: self.atlas.clone()
        })
    }
    pub fn get_size(&self, name: impl FetchSprite) -> Option<URect> {
        let position = self.names.iter().position(|n| n == name.fetch_sprite_str().as_ref())?;

        self._atlas.textures.get(position).copied()
    }
    pub fn get_index(&self, name: impl FetchSprite) -> Option<usize> {
        self.names.iter().position(|n| n == name.fetch_sprite_str().as_ref())
    }
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


#[derive(Debug, Deserialize, Clone)]
pub struct SpriteUiSheet {
    width: u32,
    height: u32,
    sprites: HashMap<String, RectPoint>
}

impl SpriteUiSheet {
    fn new() -> Self {
        let json = include_str!("../assets/sprites_css.json");

        serde_json::from_str(json).unwrap()
    }
    fn to_texture_atlas_names(&self) -> (Vec<&str>, TextureAtlasLayout) {
        let mut rects = vec![];
        let mut names = vec![];

        for (name, &rect) in &self.sprites {
            names.push(name.as_str());
            rects.push(rect.to::<URect>());
        }

        (names, TextureAtlasLayout {
            size: uvec2(self.width, self.height),
            textures: rects
        })
    }
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

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct RectPoint {
    x: u32,
    y: u32,
    width: u32,
    height: u32
}
impl From<RectPoint> for URect {
    fn from(value: RectPoint) -> Self {
        let min = uvec2(value.x, value.y);
        let max = uvec2(min.x + value.width, min.y + value.height);

        URect {
            min,
            max
        }
    }
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