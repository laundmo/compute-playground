use std::f32::consts::PI;

use bevy::{
    core::{Pod, Zeroable},
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{AsBindGroup, ShaderSize, ShaderType},
    },
};
use bevy_inspector_egui::{prelude::*, quick::ResourceInspectorPlugin};
use image::ComputePlaygroundImages;
use rand::prelude::*;

pub(crate) mod image;
mod pipeline;

const WORKGROUP_SIZE: u32 = 32;

trait ToByteBuff {
    fn to_byte_buff(&self) -> &[u8];
}

// `InspectorOptions` are completely optional
#[derive(ShaderType, Clone, Copy, Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct ShaderParams {
    size: Vec2,
    #[inspector(min = 0.0, max = 1.0, speed = 0.01)]
    diffusion: f32,
    #[inspector(min = 0.0, max = 1.0, speed = 0.01)]
    evaporation: f32,
    delta_time: f32,
}

impl Default for ShaderParams {
    fn default() -> Self {
        ShaderParams {
            size: default(),
            diffusion: 0.2,
            evaporation: 2.4,
            delta_time: default(),
        }
    }
}

#[derive(ShaderType, Clone, Copy, Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct SensorParams {
    sensor_size: i32,
    sensor_distance: f32,
    sensor_angle_between: f32,
}

impl Default for SensorParams {
    fn default() -> Self {
        Self {
            sensor_size: 2,
            sensor_distance: 12.0,
            sensor_angle_between: 0.7,
        }
    }
}

#[derive(ShaderType, Clone, Copy, Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct AgentParams {
    turn_speed: f32,
    move_speed: f32,
}

impl Default for AgentParams {
    fn default() -> Self {
        Self {
            turn_speed: 70.0,
            move_speed: 55.0,
        }
    }
}

#[derive(ShaderType, Pod, Zeroable, Copy, Clone)]
#[repr(C)]
struct Agent {
    positon: Vec2,
    angle: f32,
}

#[derive(AsBindGroup, Resource, ExtractResource, Clone, Default, Reflect, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct DataBG {
    #[uniform(0)]
    params: ShaderParams,
    #[uniform(1)]
    sensor: SensorParams,
    #[uniform(2)]
    agent: AgentParams,
}

#[derive(Resource, ExtractResource, Clone, Default)]
struct Agents {
    agents: Vec<Agent>,
}

pub struct ComputePlaygroundPlugin;

impl Plugin for ComputePlaygroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(pipeline::ShaderPipelinePlugin)
            .add_plugin(image::ImagePlugin)
            .init_resource::<DataBG>()
            .init_resource::<Agents>()
            .add_plugin(ResourceInspectorPlugin::<DataBG>::default())
            .add_plugin(ExtractResourcePlugin::<DataBG>::default())
            .add_plugin(ExtractResourcePlugin::<Agents>::default())
            .add_startup_system(setup)
            .add_systems((set_size, set_delta_time));
    }
}

fn set_delta_time(time: Res<Time>, mut data: ResMut<DataBG>) {
    data.params.delta_time = time.delta_seconds();
}

fn set_size(
    mut data: ResMut<DataBG>,
    handles: Res<ComputePlaygroundImages>,
    images: Res<Assets<Image>>,
) {
    let Some(image) = images.get(&handles.main_textures.0) else {return;};
    data.params.size = image.size();
}

fn setup(mut data: ResMut<Agents>) {
    let x_size = 1000;
    let y_size = 1000;
    data.agents = Vec::with_capacity(x_size * y_size);
    let mut rng = rand::thread_rng();
    for x in 0..x_size {
        for y in 0..y_size {
            data.agents.push(Agent {
                angle: rng.gen_range(0.0..PI * 2.0),
                positon: Vec2::new(x as f32, y as f32),
            });
        }
    }
}
