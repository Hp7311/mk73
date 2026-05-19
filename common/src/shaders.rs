use bevy::{prelude::*, render::render_resource::AsBindGroup, sprite_render::{Material2d, Material2dPlugin}};

use crate::util::InputExt;

pub struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<WorldMaterial>::default());
    }
}
#[derive(TypePath, Asset, AsBindGroup, Clone)]
pub struct WorldMaterial {
    /// within range `0.0..1.0`, specifies colour of the material
    #[uniform(0)]
    color: Vec4
}

impl WorldMaterial {
    pub fn from_srgb_u8(r: u8, g: u8, b: u8) -> Self {

        let color = vec3(
            r.to::<f32>() / u8::MAX.to::<f32>(),
            g.to::<f32>() / u8::MAX.to::<f32>(),
            b.to::<f32>() / u8::MAX.to::<f32>()
        );
        Self::new(color)
    }
    /// - `color` represents a color with each variant of `[0..1]`
    pub fn new(color: Vec3) -> Self {
        Self {
            color: color.extend(0.0)
        }
    }
}

impl Material2d for WorldMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/world.wgsl".into()
    }
}
