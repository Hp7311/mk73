#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_functions::get_world_from_local

struct Data {
    value: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    _d_padding: vec3<f32>
#endif
}

struct Vec2Data {
    value: vec2<f32>,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    _d_padding: vec2<f32>
#endif
}

@group(2) @binding(0) var<uniform> radius: Data;
@group(2) @binding(1) var<uniform> player_pos: Vec2Data;
@group(2) @binding(2) var<uniform> darkness: Data;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance_to_origin = distance(in.world_position.xy, player_pos.value);

    // discard the pixel
    if (distance_to_origin < radius.value) {
        discard;
    }

    return vec4<f32>(0.0, 0.0, 0.0, darkness.value);
}