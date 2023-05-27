use bevy::{
    core::{Pod, Zeroable},
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{AsBindGroup, ShaderSize, ShaderType},
    },
    window::WindowResized,
};
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bytemuck::{bytes_of, cast_slice};
use image::{create_image, image_from_world};

pub(crate) mod image;
mod pipeline;

const WORKGROUP_SIZE: u32 = 8;

trait ToByteBuff {
    fn to_byte_buff(&self) -> &[u8];
}

// `InspectorOptions` are completely optional
#[derive(ShaderType, Default, Clone, Copy, Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct ShaderParams {
    test: f32,
}

#[derive(ShaderType, Resource, Copy, Clone)]
#[repr(C)]
struct Agent {
    positon: Vec2,
    angle: f32,
}

#[derive(AsBindGroup, Resource)]
struct MainBindGroup {
    #[texture(0)]
    texture: Handle<Image>,
    #[uniform(1)]
    params: ShaderParams,
    #[storage(2, visibility(compute))]
    agents: Vec<Agent>,
}

impl FromWorld for MainBindGroup {
    fn from_world(world: &mut World) -> Self {
        let image = image_from_world(world);

        MainBindGroup {
            texture: image,
            params: Default::default(),
            agents: Vec::new(),
        }
    }
}

pub struct ComputePlaygroundPlugin;

impl Plugin for ComputePlaygroundPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ShaderParams::default())
            .add_plugin(pipeline::ShaderPipelinePlugin)
            .init_resource::<MainBindGroup>()
            .add_plugin(ResourceInspectorPlugin::<ShaderParams>::default());
    }
}

fn update_image(
    mut resize: EventReader<WindowResized>,
    mut image: ResMut<MainBindGroup>,
    mut images: ResMut<Assets<Image>>,
) {
    for res in resize.iter() {
        let (w, h) = (res.width, res.height);
        if w > 100.0 && h > 100.0 {
            image.texture = images.set(&image.texture, create_image(w as u32, h as u32));
        }
    }
}
