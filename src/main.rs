mod assets_map;
mod config;
mod game;
mod plugins;
mod storage;

use bevy::prelude::*;

use assets_map::AssetsMap;
use config::tuning::Tuning;
use plugins::{game_plugin::GamePlugin, menu_plugin::MenuPlugin, storage_plugin::StoragePlugin, ui_plugin::UiPlugin};

fn main() {
    let tuning = Tuning::load_or_default();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Cyber Top".into(),
                resolution: (800u32, 800u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Time::<Fixed>::from_seconds(tuning.dt as f64))
        .insert_resource(tuning)
        .insert_resource(AssetsMap::with_defaults())
        .add_plugins(GamePlugin)
        .add_plugins(MenuPlugin)
        .add_plugins(UiPlugin)
        .add_plugins(StoragePlugin)
        .run();
}
