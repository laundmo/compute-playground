struct ShaderParams {
    size: vec2<f32>,
    diffusion: f32,
    evaporation: f32,
    delta_time: f32,
}

@group(0) @binding(0)
var<uniform> params: ShaderParams;

@group(1) @binding(0)
var output_tex: texture_storage_2d<rgba8unorm, read_write>;

@group(1) @binding(1)
var input_tex : texture_2d<f32>;

@compute @workgroup_size(8,8,1)
fn image(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(invocation_id.xy);
    var original = textureLoad(input_tex, location, 0);

    var sum = vec4<f32>(0.0);
    for (var r = -1; r <= 1; r++) {
        for (var c = -1; c <= 1; c++) {
            let new_loc = location + vec2<i32>(r, c);
            if all(new_loc == location) {
                continue;
            } else {
                sum += textureLoad(input_tex, new_loc, 0);
            }
        }
    }
    var avg = sum / 9.0;

    var diffused_color = mix(original, avg, params.diffusion * params.delta_time);
    var diffuse_evaporated_color = max(vec4<f32>(0.0), diffused_color - params.evaporation * params.delta_time);

    textureStore(output_tex, location, diffuse_evaporated_color);
}