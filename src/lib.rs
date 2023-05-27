use std::ops::Deref;

use bevy::{
    core::{Pod, Zeroable},
    input::{mouse::MouseButtonInput, ButtonState},
    math::Vec4Swizzles,
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{Extent3d, ShaderType, TextureDimension, TextureFormat, TextureUsages},
    },
    window::{PrimaryWindow, WindowResized},
};
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;

mod pipeline;

const WORKGROUP_SIZE: u32 = 8;

// `InspectorOptions` are completely optional
#[derive(
    ShaderType,
    Pod,
    Zeroable,
    Default,
    Clone,
    Copy,
    Resource,
    Reflect,
    InspectorOptions,
    ExtractResource,
)]
#[reflect(Resource, InspectorOptions)]
#[repr(C)]
struct ShaderParams {
    extents: Vec4,
    size: Vec2,
    pad: Vec2,
}

#[derive(Resource, Default)]
struct ZoomEvent(Vec4);

pub struct ComputePlaygroundPlugin;

impl Plugin for ComputePlaygroundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShaderImage>()
            .insert_resource(ShaderParams {
                extents: Vec4::new(-1.0, 1.0, -1.0, 1.0),
                size: Vec2::new(1000.0, 1000.0),
                ..default()
            })
            .add_event::<ZoomEvent>()
            .add_plugin(pipeline::ShaderPipelinePlugin)
            .add_plugin(ResourceInspectorPlugin::<ShaderParams>::default())
            .add_plugin(ExtractResourcePlugin::<ShaderParams>::default())
            .add_plugin(ExtractResourcePlugin::<ShaderImage>::default())
            .add_systems((update_image, zoom, zoom_smooth).chain());
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (1.0 - t) * a + t * b
}

fn inv_lerp(a: f32, b: f32, v: f32) -> f32 {
    (v - a) / (b - a)
}

fn remap(i: Vec2, o: Vec2, v: f32) -> f32 {
    lerp(o.x, o.y, inv_lerp(i.x, i.y, v))
}

fn zoom_smooth(
    mut target: Local<Vec4>,
    mut zoom_target: EventReader<ZoomEvent>,
    mut shader_params: ResMut<ShaderParams>,
    mut timer: Local<Timer>,
    time: Res<Time>,
) {
    if !zoom_target.is_empty() {
        *timer = Timer::from_seconds(1.0, TimerMode::Once);
        *target = zoom_target.iter().last().unwrap().0;
    }
    if timer.duration().as_secs_f32() != 0.0 && !timer.finished() {
        // fix: only run when timer runs
        timer.tick(time.delta());
        shader_params.extents = shader_params.extents.lerp(*target, timer.percent());
    }
}

fn zoom(
    shader_params: Res<ShaderParams>,
    mut mouseclick: EventReader<MouseButtonInput>,
    mut zoom_target: EventWriter<ZoomEvent>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let window = window.single();
    for evt in mouseclick.iter() {
        if let ButtonState::Released = evt.state {
            let pos = window.cursor_position().unwrap();
            let initial: Vec4 = shader_params.extents;
            let winsize = Vec2::new(window.width(), window.height());

            let pos_x = remap(Vec2::new(0.0, winsize.x), initial.xy(), pos.x);
            let pos_y = remap(Vec2::new(0.0, winsize.y), initial.zw(), pos.y);
            let x_width;
            let y_width;
            match evt.button {
                MouseButton::Left => {
                    x_width = (initial.y - initial.x) / 2.8;
                    y_width = (initial.w - initial.z) / 2.8;
                }
                MouseButton::Right => {
                    x_width = (initial.y - initial.x) / 2.0 * 1.8;
                    y_width = (initial.w - initial.z) / 2.0 * 1.8;
                }
                _ => return,
            };
            zoom_target.send(ZoomEvent(Vec4::new(
                pos_x - x_width,
                pos_x + x_width,
                pos_y - y_width,
                pos_y + y_width,
            )));
        }
    }
}

#[derive(Resource, Clone, Deref, ExtractResource)]
struct ShaderImage(pub Handle<Image>);

fn create_image(width: u32, height: u32) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    image
}

impl FromWorld for ShaderImage {
    fn from_world(world: &mut World) -> Self {
        let mut win = world.query::<&Window>();
        let win = win.single(world);
        let image = create_image(win.width() as u32, win.height() as u32);
        let image = world.resource_mut::<Assets<Image>>().add(image);
        world.spawn(SpriteBundle {
            texture: image.clone(),
            ..default()
        });
        ShaderImage(image)
    }
}

fn update_image(
    mut resize: EventReader<WindowResized>,
    mut image: ResMut<ShaderImage>,
    mut images: ResMut<Assets<Image>>,
    mut shader_params: ResMut<ShaderParams>,
) {
    for res in resize.iter() {
        let (w, h) = (res.width, res.height);
        if w > 100.0 && h > 100.0 {
            image.0 = images.set(&image.0, create_image(w as u32, h as u32));

            let o = shader_params.extents;
            let aspect_ratio = w / h;
            let total_width = (o.w.abs() + o.z.abs()) * aspect_ratio;
            shader_params.extents = Vec4::new(-(total_width / 2.0), total_width / 2.0, o.z, o.w);
            shader_params.size = Vec2::new(w, h);
        }
    }
}
