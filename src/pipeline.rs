use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::FallbackImage,
        RenderApp, RenderSet,
    },
};

use crate::{MainBindGroup, ShaderParams, ToByteBuff, WORKGROUP_SIZE};

pub(crate) struct ShaderPipelinePlugin;
impl Plugin for ShaderPipelinePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        let render_device = render_app.world.resource::<RenderDevice>();
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("configuration uniform buffer"),
            size: std::mem::size_of::<ShaderParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        render_app
            .init_resource::<ShaderPipeline>()
            .init_resource::<FallbackImage>()
            .add_system(queue_bind_group.in_set(RenderSet::Queue));
        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("compute_shader", ShaderNode::default());
        render_graph.add_node_edge(
            "compute_shader",
            bevy::render::main_graph::node::CAMERA_DRIVER,
        );
    }
}

#[derive(Resource)]
pub(crate) struct ShaderPipeline {
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for ShaderPipeline {
    fn from_world(world: &mut World) -> Self {
        let bind_group_layout = MainBindGroup::bind_group_layout(world.resource::<RenderDevice>());

        let pipeline_cache = world.resource::<PipelineCache>();

        let shader = world.resource::<AssetServer>().load("shaders/compute.wgsl");

        let init_pipeline = pipeline_cache.queue_compute_pipeline(compute_descriptor(
            &bind_group_layout,
            "init",
            shader.clone(),
        ));
        let update_pipeline = pipeline_cache.queue_compute_pipeline(compute_descriptor(
            &bind_group_layout,
            "update",
            shader,
        ));

        Self {
            init_pipeline,
            update_pipeline,
            bind_group_layout,
        }
    }
}

fn compute_descriptor(
    layout: &BindGroupLayout,
    entry: &str,
    shader: Handle<Shader>,
) -> ComputePipelineDescriptor {
    let entrypoint = Cow::from(entry.to_owned());
    ComputePipelineDescriptor {
        label: Some(entrypoint.clone()),
        layout: vec![layout.clone()],
        push_constant_ranges: vec![],
        shader,
        shader_defs: vec![],
        entry_point: entrypoint,
    }
}

#[derive(Resource)]
struct ShaderBindGroup(pub PreparedBindGroup<()>);

fn queue_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<ShaderPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    main_bindgroup: Res<MainBindGroup>,
) {
    let Ok(bind_group) = main_bindgroup.as_bind_group(
        &pipeline.bind_group_layout,
        &render_device,
        &gpu_images,
        &fallback_image,
    ) else { info!("bind group prepare failed"); return };
    commands.insert_resource(ShaderBindGroup(bind_group))
}

#[derive(Default)]
pub enum ShaderState {
    #[default]
    Loading,
    Init,
    Update,
}

#[derive(Default)]
pub struct ShaderNode {
    state: ShaderState,
}

impl Node for ShaderNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<ShaderPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        match self.state {
            ShaderState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = ShaderState::Init;
                }
            }
            ShaderState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = ShaderState::Update;
                }
            }
            ShaderState::Update => (),
        };
    }
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let bind_group = &world.resource::<ShaderBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ShaderPipeline>();
        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_bind_group(0, &bind_group.bind_group, &[]);

        let main_bg = world.resource::<MainBindGroup>();
        let images = world.resource::<RenderAssets<Image>>();
        let Some(image) = images.get(&main_bg.texture) else {return Ok(());};

        match self.state {
            ShaderState::Loading | ShaderState::Init => (),
            ShaderState::Update => {
                let update_pipline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipline);
                pass.dispatch_workgroups(
                    image.size.x as u32 / WORKGROUP_SIZE,
                    image.size.y as u32 / WORKGROUP_SIZE,
                    1,
                );
            }
        };
        Ok(())
    }

    fn input(&self) -> Vec<SlotInfo> {
        Vec::new()
    }

    fn output(&self) -> Vec<SlotInfo> {
        Vec::new()
    }
}
