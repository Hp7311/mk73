use std::collections::HashMap;

use proc_macro_crate::FoundCrate;
use serde::Deserialize;
use syn::Path;

// same in client but deleted unnecessary parts

#[derive(Debug, Deserialize)]
pub struct SpriteSheet {
    pub frames: HashMap<String, SheetCell>
}

impl SpriteSheet {
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.frames.contains_key(name)
    }
    pub(crate) fn get_size(&self, name: &str) -> Option<Rect2> {
        let frame = self.frames.get(name)?.frame;

        Some(Rect2 { w: frame.w, h: frame.h })
    }
}
#[derive(Debug, Deserialize)]
pub struct SheetCell {
    pub frame: Rect2
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Rect2 {
    pub w: u32,
    pub h: u32
}

    /// - `path` is relative to common root without the :: or `crate`
pub fn absolute_path(path: &str) -> Path {
    let path = match proc_macro_crate::crate_name("common").unwrap() {
        FoundCrate::Itself => format!("crate::{path}"),
        FoundCrate::Name(name) => format!("{name}::{path}")
    };

    syn::parse_str(&path).unwrap()
}