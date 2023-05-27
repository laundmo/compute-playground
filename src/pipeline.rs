use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo},
        render_resource::{encase::private::WriteInto, *},
        renderer::{RenderContext, RenderDevice, RenderQueue},
        RenderApp, RenderSet,
    },
};

use crate::{ShaderImage, ShaderParams, WORKGROUP_SIZE};

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
            .insert_resource(ParamsBuff(buffer))
            .add_system(prepare_params::<ShaderParams, ParamsBuff>.in_set(RenderSet::Prepare))
            //.add_system(prepare_params::<ShaderParams, ParamsBuff>.in_set(RenderSet::Prepare))
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
        let bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Shader Playground Bind Group Layout"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::StorageTexture {
                                access: StorageTextureAccess::ReadWrite,
                                format: TextureFormat::Rgba8Unorm,
                                view_dimension: TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

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
struct ShaderBindGroup(pub BindGroup);

#[derive(Resource, Deref, DerefMut)]
struct ParamsBuff(Buffer);

#[derive(Resource, Deref, DerefMut)]
struct ParticleDataBuff(Buffer);

fn prepare_params<
    T: ShaderType + WriteInto + Resource,
    D: std::ops::Deref<Target = Buffer> + Resource,
>(
    source: Res<T>,
    dest: Res<D>,
    render_queue: Res<RenderQueue>,
) {
    ShaderParams::assert_uniform_compat();
    let mut buffer = encase::UniformBuffer::new(Vec::new());
    buffer.write::<T>(&source).unwrap();

    render_queue.write_buffer(&*dest, 0, buffer.as_ref().as_slice());
}

fn queue_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<ShaderPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    shader_image: Res<ShaderImage>,
    config: ResMut<ParamsBuff>,
) {
    let view = &gpu_images[&shader_image.0];
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("Shader Playground Bind Group"),
        layout: &pipeline.bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view.texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: config.as_entire_binding(),
            },
        ],
    });
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
        let texture_bind_group = &world.resource::<ShaderBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ShaderPipeline>();
        let image = world.resource::<ShaderImage>();
        let images = world.resource::<RenderAssets<Image>>();
        let Some(image) = images.get(image) else {return Ok(());};
        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_bind_group(0, texture_bind_group, &[]);
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
