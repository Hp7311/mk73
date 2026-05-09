use bevy::{prelude::*, render::render_resource::AsBindGroup, sprite_render::{Material2d, Material2dPlugin}};

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
    pub color: Vec3
}


impl Material2d for WorldMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/world.wgsl".into()
    }
}
