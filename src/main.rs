#![allow(clippy::redundant_field_names)]
use bevy::{prelude::*, render::camera::ScalingMode, window::PresentMode};

pub const CLEAR: Color = Color::rgb(0.1, 0.1, 0.1);
pub const RESOLUTION: f32 = 16.0 / 9.0;
pub const TILE_SIZE: f32 = 0.075;

mod ascii;
mod combat;
mod debug;
mod fadeout;
mod player;
mod tilemap;
mod audio;
mod graphics;
mod start_menu;
mod npc;

use graphics::GraphicsPlugin;
use ascii::AsciiPlugin;
use combat::CombatPlugin;
use debug::DebugPlugin;
use fadeout::FadeoutPlugin;
use player::PlayerPlugin;
use tilemap::TileMapPlugin;
use audio::GameAudioPlugin;
use start_menu::MainMenuPlugin;
use npc::NpcPlugin;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum GameState {
    StartMenu,
    Overworld,
    Combat,
}

fn main() {
    App::new()
        .add_state(GameState::StartMenu)
        .insert_resource(ClearColor(CLEAR))
        .insert_resource(WindowDescriptor {
            width: 1600.0,
            height: 900.0,
            title: "Dreadblaze".to_string(),
            present_mode: PresentMode::Fifo,
            resizable: false,
            ..Default::default()
        })
        .add_startup_system(spawn_camera)
        .add_plugins(DefaultPlugins)
        .add_plugin(PlayerPlugin)
        .add_plugin(CombatPlugin)
        .add_plugin(AsciiPlugin)
        .add_plugin(DebugPlugin)
        .add_plugin(TileMapPlugin)
        .add_plugin(FadeoutPlugin)
        .add_plugin(GameAudioPlugin)
        .add_plugin(GraphicsPlugin)
        .add_plugin(MainMenuPlugin)
        .add_plugin(NpcPlugin)
        .run();
}

#[derive(Component)]
pub struct MainCamera;

fn spawn_camera(mut commands: Commands) {
    let mut camera = OrthographicCameraBundle::new_2d();

    camera.orthographic_projection.top = 1.0;
    camera.orthographic_projection.bottom = -1.0;

    camera.orthographic_projection.right = 1.0 * RESOLUTION;
    camera.orthographic_projection.left = -1.0 * RESOLUTION;

    camera.orthographic_projection.scaling_mode = ScalingMode::None;

    commands.spawn_bundle(camera);
}
