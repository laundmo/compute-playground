#import bevy_pbr::utils

struct ShaderParams {
    size: vec2<f32>,
    diffusion: f32,
    evaporation: f32,
    delta_time: f32,
    turn_speed: f32,
}

struct SensorParams {
    sensor_size: i32,
    sensor_distance: f32,
    sensor_angle_between: f32,
}

struct AgentParams {
    turn_speed: f32,
    move_speed: f32,
}

@group(0) @binding(0)
var<uniform> params: ShaderParams;

@group(0) @binding(1)
var<uniform> sens: SensorParams;

@group(0) @binding(2)
var<uniform> p_agent: AgentParams;

@group(1) @binding(0)
var output_tex: texture_storage_2d<rgba8unorm, read_write>;

@group(1) @binding(1)
var input_tex : texture_2d<f32>;

struct Agent {
    position: vec2<f32>,
    angle: f32
}
struct Agents {
    agents: array<Agent>,
}

@group(2) @binding(0)
var<storage, read_write> agents: Agents;


fn hash(value: u32) -> u32 {
    var state = value;
    state = state ^ 2747636419u;
    state = state * 2654435769u;
    state = state ^ (state >> 16u);
    state = state * 2654435769u;
    state = state ^ (state >> 16u);
    state = state * 2654435769u;
    return state;
}

fn randomFloat(value: u32) -> f32 {
    return f32(hash(value)) / 4294967295.0;
}

fn randomFloat01(value: u32) -> f32 {
    return max(0.0, min(1.0, randomFloat(value)));
}


fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return (1.0 - t) * a + t * b;
}

fn inv_lerp(a: f32, b: f32, v: f32) -> f32 {
    return (v - a) / (b - a);
}

fn remap(i_min: f32, i_max: f32, o_min: f32, o_max: f32, v: f32) -> f32 {
    return lerp(o_min, o_max, inv_lerp(i_min, i_max, v));
}

@compute @workgroup_size(32,1,1) 
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
}

fn sensor(agent: Agent, sensor_angle: f32) -> f32 {
    let sensor_angle = agent.angle + sensor_angle;
    let sensor_dir = vec2<f32>(cos(sensor_angle), sin(sensor_angle));
    let sensor_mid = vec2<i32>(agent.position + sensor_dir * sens.sensor_distance);

    var sum = 0.0;
    for (var r = -sens.sensor_size; r <= sens.sensor_size; r++) {
        for (var c = -sens.sensor_size; c <= sens.sensor_size; c++) {
            let new_loc = sensor_mid + vec2<i32>(r, c);
            sum += textureLoad(input_tex, new_loc, 0).x;
        }
    }
    return sum;
}

@compute @workgroup_size(32,1,1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = invocation_id.x;
    var agent = agents.agents[location];
    var random = randomFloat(u32(agent.position.x) * hash(u32(agent.position.y)) + hash(invocation_id.x));
    var direction = vec2<f32>(cos(agent.angle), sin(agent.angle));
    var new_pos = agent.position + direction * p_agent.move_speed * params.delta_time;

    if new_pos.x < 0.0 || new_pos.x >= params.size.x {
        new_pos = min(new_pos, max(vec2<f32>(0.0), new_pos));
        direction = vec2<f32>(-direction.x, direction.y);
        agents.agents[location].angle = atan2(direction.x, direction.y);
    }
    if new_pos.y < 0.0 || new_pos.y >= params.size.y {
        new_pos = min(new_pos, max(vec2<f32>(0.0), new_pos));
        direction = vec2<f32>(direction.x, -direction.y);
        agents.agents[location].angle = atan2(direction.x, direction.y);
    }

    agents.agents[location].position = new_pos;

    var w_forward = sensor(agent, 0.0);
    var w_left = sensor(agent, sens.sensor_angle_between);
    var w_right = sensor(agent, -sens.sensor_angle_between);

    var random_steer = randomFloat01(u32(random));

    if w_forward > w_left && w_forward > w_right {
    } else if w_forward < w_left && w_forward < w_right {
        agents.agents[location].angle += (random_steer - 0.5) * 0.2 * p_agent.turn_speed * params.delta_time;
    } else if w_right > w_left {
        agents.agents[location].angle -= random_steer * p_agent.turn_speed * params.delta_time;
    } else if w_left > w_right {
        agents.agents[location].angle += random_steer * p_agent.turn_speed * params.delta_time;
    }
    var color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    textureStore(output_tex, vec2<i32>(agent.position), color);
}
