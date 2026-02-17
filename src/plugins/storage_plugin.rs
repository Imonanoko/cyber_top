use bevy::prelude::*;

use crate::config::tuning::Tuning;
use crate::storage::sqlite_repo::SqliteRepo;

/// Persisted tokio runtime for sync DB calls outside startup.
#[derive(Resource)]
pub struct TokioRuntime(pub tokio::runtime::Runtime);

pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_storage);
    }
}

fn init_storage(world: &mut World) {
    let db_path = Tuning::data_dir().join("cyber_top.db");
    info!("Initializing SQLite at {:?}", db_path);

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    match rt.block_on(SqliteRepo::new(&db_path)) {
        Ok(repo) => {
            info!("SQLite initialized successfully");
            world.insert_resource(repo);
        }
        Err(e) => {
            error!("Failed to initialize SQLite: {e}");
        }
    }
    // Keep runtime alive for sync DB calls in design screens
    world.insert_resource(TokioRuntime(rt));
}
