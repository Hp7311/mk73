use bevy::{
    prelude::*,
    render::render_resource::AsBindGroup,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};

pub struct ShadersPlugin;

impl Plugin for ShadersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<DivingOverlay>::default());
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub(crate) struct DivingOverlay {
    #[uniform(0)]
    pub radius: f32,
    #[uniform(0)]
    pub _r_padding1: f32,
    #[uniform(0)]
    pub _r_padding2: f32,
    #[uniform(0)]
    pub _r_padding3: f32,
    #[uniform(1)]
    pub player_pos: Vec2,

    #[uniform(1)]
    pub _p_padding: Vec2,

    #[uniform(2)]
    pub darkness: f32,
    #[uniform(2)]
    pub _d_padding1: f32,
    #[uniform(2)]
    pub _d_padding2: f32,
    #[uniform(2)]
    pub _d_padding3: f32,
}

impl Material2d for DivingOverlay {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/diving_overlay.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}
