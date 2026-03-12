#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_functions::get_world_from_local


@group(2) @binding(0) var<uniform> radius: f32;
@group(2) @binding(1) var<uniform> player_pos: vec2<f32>;
@group(2) @binding(2) var<uniform> darkness: f32;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance_to_origin = distance(in.world_position.xy, player_pos);

    // discard the pixel
    if (distance_to_origin < radius) {
        discard;
    }

    return vec4<f32>(0.0, 0.0, 0.0, darkness);
}