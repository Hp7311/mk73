use bevy::{
    prelude::*, render::render_resource::AsBindGroup, sprite_render::{AlphaMode2d, Material2d, Material2dPlugin}
};

pub struct ShadersPlugin;

impl Plugin for ShadersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<DivingOverlay>::default());
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub(crate) struct DivingOverlay {
    #[uniform(0)]
    radius: f32
}

impl DivingOverlay {
    pub fn new(radius: f32) -> Self {
        Self {
            radius
        }
    }
}

impl Material2d for DivingOverlay {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/diving_overlay.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}