#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_functions::get_world_from_local

struct DivingMaterial {
    radius: f32
}

@group(2) @binding(0) var<uniform> diving_material: DivingMaterial;

// assumes that origin is (0.5, 0.5)
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance_to_origin = distance(in.uv, vec2<f32>(0.5));

    // discard the pixel
    if (distance_to_origin < diving_material.radius) {
        discard;
    }

    return vec4<f32>(0.0, 0.0, 0.0, 0.9);
}