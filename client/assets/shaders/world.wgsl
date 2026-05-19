#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> color: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // let sampled = textureSample(world_image, world_image_sampler, in.uv);
    
    // let color = vec3<f32>(1.3 / 255.0, 14.0 / 255.0, 41.0 / 255.0);
    // mk48 stuff
    // let deep = mix(vec3<f32>(0.0, 0.0331, 0.171) * 0.82, vec3<f32>(0.0, 0.0331, 0.0763), 0.0);
    // let shallow = mix(vec3<f32>(0.0331, 0.113, 0.242) * 0.9, vec3<f32>(0.0, 0.05, 0.115), 0.6);
    // , arctic) * (mix(light, waterLight, 0.6));
    // let shallow: vec3<f32> = mix(vec3(0.0331, 0.113, 0.242) * 0.9, vec3(0.0, 0.05, 0.115));//, arctic) * waterLight;
    // var w: vec3<f32> = mix(deep, shallow, pow(0.005, abs(sandHeight - height))); // Deep to shallow water.

    // let waveN: vec3<f32> = normalize(cross(vec3(uDerivative, 0.0, dFdx(wn.y)), vec3(0.0, uDerivative, dFdy(wn.y))));

    // let viewDir = vec3(0.0, 0.0, 1.0);
    // var r: f32 = clamp(dot(reflect(-uWaterSun, waveN), viewDir), 0.0, 1.0);

    // // r = pow(r, 16.0);
    // r *= r;
    // r *= r;
    // r *= r;
    // r *= r;

    // // Add specular highlight of waves to water color.
    // w += r * smoothstep(-sandHeight, -(sandHeight - 0.25), -height) * 0.3 * (sun * 0.85 + 0.15);

    // Foam appears near surface and is uniform width.
    // let t: f32 = sandHeight - height;
    // let foam: f32 = smoothstep(-0.05, 0.0, -t / (1.0001 - N.z));
    // let foamColor: f32 = mix(mix(shallow, beach, 0.5), vec3(0.5), float(ocean)) * (foam * foam);
    // w = max(w, foamColor * light);

    // Antialias foam and sand.
    // let delta = uDerivative * 0.015;
    // let delta = 0.0;
    // fragColor = vec4(mix(s, w, smoothstep(-delta, delta, t)), 1.0);

    return vec4(color.rgb, 1.0);                        
}