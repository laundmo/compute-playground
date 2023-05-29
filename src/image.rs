use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
    window::WindowResized,
};

pub(super) struct ImagePlugin;
impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ComputePlaygroundImages>()
            .add_plugin(ExtractResourcePlugin::<ComputePlaygroundImages>::default())
            .add_system(update_image);
    }
}

#[derive(Resource, ExtractResource, Clone)]
pub(crate) struct ComputePlaygroundImages {
    pub(crate) main_textures: (Handle<Image>, Handle<Image>),
}

#[derive(Component, Default)]
struct MainImageMarker;

impl FromWorld for ComputePlaygroundImages {
    fn from_world(world: &mut World) -> Self {
        let mut win = world.query::<&Window>();
        let win = win.single(world);
        let (w, h) = (win.width() as u32, win.height() as u32);
        let mut image_assets = world.resource_mut::<Assets<Image>>();

        let imagea = image_assets.add(create_image(w, h));

        let imageb = image_assets.add(create_image(w, h));

        world.spawn((
            SpriteBundle {
                texture: imageb.clone(),
                ..default()
            },
            MainImageMarker,
        ));
        ComputePlaygroundImages {
            main_textures: (imagea, imageb),
        }
    }
}

pub fn create_image(width: u32, height: u32) -> Image {
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

fn update_image(
    mut resize: EventReader<WindowResized>,
    mut handles: ResMut<ComputePlaygroundImages>,
    mut images: ResMut<Assets<Image>>,
) {
    for res in resize.iter() {
        let (w, h) = (res.width, res.height);
        if w > 100.0 && h > 100.0 {
            handles.main_textures.0 =
                images.set(&handles.main_textures.0, create_image(w as u32, h as u32));
            handles.main_textures.1 =
                images.set(&handles.main_textures.1, create_image(w as u32, h as u32));
        }
    }
}

fn flip(handles: Res<ComputePlaygroundImages>, mut images: ResMut<Assets<Image>>) {
    let imagea = images.get(&handles.main_textures.0).unwrap().data.clone();
    let imageb = images.get(&handles.main_textures.1).unwrap().data.clone();
    images.get_mut(&handles.main_textures.0).unwrap().data = imageb;
    images.get_mut(&handles.main_textures.0).unwrap().data = imagea;
}
