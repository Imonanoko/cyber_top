mod assets_map;
mod config;
mod game;
mod plugins;
mod storage;

use bevy::prelude::*;

use config::tuning::Tuning;
use plugins::{design_plugin::DesignPlugin, game_plugin::GamePlugin, map_design_plugin::MapDesignPlugin, menu_plugin::MenuPlugin, storage_plugin::StoragePlugin, ui_plugin::UiPlugin};

fn main() {
    let tuning = Tuning::load_or_default();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Cyber Top".into(),
                resolution: (900u32, 1200u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Time::<Fixed>::from_seconds(tuning.dt as f64))
        .insert_resource(tuning)
        .add_plugins(GamePlugin)
        .add_plugins(MenuPlugin)
        .add_plugins(UiPlugin)
        .add_plugins(StoragePlugin)
        .add_plugins(DesignPlugin)
        .add_plugins(MapDesignPlugin)
        .run();
}
