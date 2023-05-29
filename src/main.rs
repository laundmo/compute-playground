#![doc = include_str!("../README.md")]
use bevy::{prelude::*, window::WindowResolution};
use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};
use compute_playground::*;

const BACKGROUND_COLOR: Color = Color::BLACK;

fn main() {
    let mut app = App::new();

    app.insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        position: WindowPosition::At(IVec2 { x: 0, y: 0 }),
                        resolution: WindowResolution::new(1000.0, 1000.0),
                        canvas: Some("#newtons_fractal".to_owned()),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    watch_for_changes: true,
                    ..default()
                }),
        )
        .add_plugin(ScreenDiagnosticsPlugin::default())
        .add_plugin(ScreenFrameDiagnosticsPlugin)
        .add_startup_system(spawn_camera)
        .add_plugin(ComputePlaygroundPlugin);

    app.run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
