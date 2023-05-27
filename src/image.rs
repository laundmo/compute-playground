use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
    window::WindowResized,
};

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

pub fn image_from_world(world: &mut World) -> Handle<Image> {
    let mut win = world.query::<&Window>();
    let win = win.single(world);
    let image = create_image(win.width() as u32, win.height() as u32);
    let image = world.resource_mut::<Assets<Image>>().add(image);
    world.spawn(SpriteBundle {
        texture: image.clone(),
        ..default()
    });
    image
}
