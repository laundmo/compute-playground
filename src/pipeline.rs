use std::borrow::Cow;

use bevy::{
    core::FrameCount,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo},
        render_resource::{
            encase::StorageBuffer, AsBindGroup, BindGroup, BindGroupDescriptor, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
            BindingType, Buffer, BufferBindingType, BufferInitDescriptor, BufferUsages,
            CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache, PreparedBindGroup, SamplerBindingType,
            SamplerDescriptor, ShaderStages, StorageTextureAccess, TextureFormat,
            TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderContext, RenderDevice},
        texture::FallbackImage,
        RenderApp, RenderSet,
    },
};

use crate::{image::ComputePlaygroundImages, Agents, DataBG, ShaderParams, WORKGROUP_SIZE};

pub(crate) struct ShaderPipelinePlugin;
impl Plugin for ShaderPipelinePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<ShaderPipeline>()
            .init_resource::<FallbackImage>()
            .insert_resource(AgentsBuffer(None))
            .add_system(
                prepare_agents
                    .in_set(RenderSet::Prepare)
                    .run_if(resource_changed::<Agents>()),
            )
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
    image_pipeline: CachedComputePipelineId,
    texture_bind_group_layout: BindGroupLayout,
    data_bind_group_layout: BindGroupLayout,
    agents_bind_group_layout: BindGroupLayout,
}

impl FromWorld for ShaderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = &world.resource::<RenderDevice>();

        let data_bind_group_layout = DataBG::bind_group_layout(render_device);

        let texture_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("LayoutTextureBindGroup"),
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
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        let agents_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("LayoutTextureBindGroup"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let pipeline_cache = world.resource::<PipelineCache>();

        let asset_server = &world.resource::<AssetServer>();

        let main_shader = asset_server.load("shaders/compute.wgsl");
        let blurr_shader = asset_server.load("shaders/blurr.wgsl");

        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("init".to_owned()).clone()),
            layout: vec![
                data_bind_group_layout.clone(),
                texture_bind_group_layout.clone(),
                agents_bind_group_layout.clone(),
            ],
            push_constant_ranges: vec![],
            shader: main_shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init".to_owned()),
        });

        let flip_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("init".to_owned()).clone()),
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: vec![],
            shader: main_shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init".to_owned()),
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline({
            ComputePipelineDescriptor {
                label: Some(Cow::from("update".to_owned()).clone()),
                layout: vec![
                    data_bind_group_layout.clone(),
                    texture_bind_group_layout.clone(),
                    agents_bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![],
                shader: main_shader,
                shader_defs: vec![],
                entry_point: Cow::from("update".to_owned()),
            }
        });
        let image_pipeline = pipeline_cache.queue_compute_pipeline({
            ComputePipelineDescriptor {
                label: Some(Cow::from("image".to_owned()).clone()),
                layout: vec![
                    data_bind_group_layout.clone(),
                    texture_bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![],
                shader: blurr_shader,
                shader_defs: vec![],
                entry_point: Cow::from("image".to_owned()),
            }
        });

        Self {
            init_pipeline,
            update_pipeline,
            image_pipeline,
            texture_bind_group_layout,
            data_bind_group_layout,
            agents_bind_group_layout,
        }
    }
}

#[derive(Resource)]
struct ShaderBindGroups {
    pub texture_a_bind_group: BindGroup,
    pub texture_b_bind_group: BindGroup,
    pub agents_bind_group: BindGroup,
    pub data_bind_group: PreparedBindGroup<()>,
}

#[derive(Resource)]
struct AgentsBuffer(Option<Buffer>);

fn prepare_agents(
    agents: Res<Agents>,
    mut agents_buffer: ResMut<AgentsBuffer>,
    render_device: Res<RenderDevice>,
) {
    let mut buffer = StorageBuffer::new(Vec::new());
    buffer.write(&agents.agents).unwrap();
    agents_buffer.0 = Some(
        render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            contents: buffer.as_ref(),
            usage: BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::STORAGE,
        }),
    );
}

fn queue_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<ShaderPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    main_bindgroup: Res<DataBG>,
    agents_buffer: Res<AgentsBuffer>,
    images: Res<ComputePlaygroundImages>,
) {
    let viewa = &gpu_images[&images.main_textures.0];
    let viewb = &gpu_images[&images.main_textures.1];

    let Ok(bind_group) = main_bindgroup.as_bind_group(
        &pipeline.data_bind_group_layout,
        &render_device,
        &gpu_images,
        &fallback_image,
    ) else { info!("bind group prepare failed"); return };

    commands.insert_resource(ShaderBindGroups {
        texture_a_bind_group: render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("TextureBindGroup"),
            layout: &pipeline.texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&viewa.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&viewb.texture_view),
                },
            ],
        }),
        texture_b_bind_group: render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("TextureBindGroup"),
            layout: &pipeline.texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&viewb.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&viewa.texture_view),
                },
            ],
        }),
        agents_bind_group: render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("TextureBindGroup"),
            layout: &pipeline.agents_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: agents_buffer.0.as_ref().unwrap().as_entire_binding(),
            }],
        }),
        data_bind_group: bind_group,
    })
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
        let bind_groups = &world.resource::<ShaderBindGroups>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ShaderPipeline>();
        let data = world.resource::<DataBG>();
        let (w, h) = (data.params.size.x as u32, data.params.size.y as u32);

        let frames = world.resource::<FrameCount>();
        let texturesa;
        let texturesb;
        if frames.0 % 2 == 0 {
            texturesa = &bind_groups.texture_a_bind_group;
            texturesb = &bind_groups.texture_b_bind_group;
        } else {
            texturesa = &bind_groups.texture_b_bind_group;
            texturesb = &bind_groups.texture_a_bind_group;
        }
        // First pass: using agents
        {
            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_bind_group(0, &bind_groups.data_bind_group.bind_group, &[]);
            pass.set_bind_group(1, texturesa, &[]);
            pass.set_bind_group(2, &bind_groups.agents_bind_group, &[]);

            let agents_len = world.resource::<Agents>().agents.len();

            match self.state {
                ShaderState::Loading | ShaderState::Init => (),
                ShaderState::Update => {
                    let update_pipline = pipeline_cache
                        .get_compute_pipeline(pipeline.update_pipeline)
                        .unwrap();
                    pass.set_pipeline(update_pipline);
                    pass.dispatch_workgroups(agents_len as u32 / WORKGROUP_SIZE, 1, 1);
                }
            };
        }

        // second pass: not using agents, full image
        {
            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_bind_group(0, &bind_groups.data_bind_group.bind_group, &[]);
            pass.set_bind_group(1, texturesb, &[]);
            let image_pipeline = pipeline_cache
                .get_compute_pipeline(pipeline.image_pipeline)
                .unwrap();
            pass.set_pipeline(image_pipeline);
            pass.dispatch_workgroups(w / 8, h / 8, 1);
        }
        Ok(())
    }

    fn input(&self) -> Vec<SlotInfo> {
        Vec::new()
    }

    fn output(&self) -> Vec<SlotInfo> {
        Vec::new()
    }
}
