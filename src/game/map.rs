use serde::{Deserialize, Serialize};

/// A map definition: arena size + placed items on a grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapSpec {
    pub id: String,
    pub name: String,
    pub arena_radius: f32,
    pub placements: Vec<MapPlacement>,
}

impl MapSpec {
    pub fn default_arena() -> Self {
        Self {
            id: "default_arena".into(),
            name: "Default Arena".into(),
            arena_radius: 12.0,
            placements: vec![],
        }
    }
}

/// A single placed item on the grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapPlacement {
    pub grid_x: i32,
    pub grid_y: i32,
    pub item: MapItem,
}

/// Types of items that can be placed on the map grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MapItem {
    Obstacle,
    GravityDevice,
    SpeedBoost,
    DamageBoost,
}

impl MapItem {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Obstacle => "Obstacle",
            Self::GravityDevice => "Gravity",
            Self::SpeedBoost => "Speed Boost",
            Self::DamageBoost => "Dmg Boost",
        }
    }

    pub fn color(self) -> bevy::prelude::Color {
        match self {
            Self::Obstacle => bevy::prelude::Color::srgba(0.5, 0.5, 0.5, 1.0),
            Self::GravityDevice => bevy::prelude::Color::srgba(0.6, 0.2, 0.8, 1.0),
            Self::SpeedBoost => bevy::prelude::Color::srgba(0.2, 0.8, 0.3, 1.0),
            Self::DamageBoost => bevy::prelude::Color::srgba(0.8, 0.2, 0.2, 1.0),
        }
    }
}

/// Grid cell size in world units.
pub const GRID_CELL_SIZE: f32 = 0.5;

/// Check if a grid cell is within the arena circle.
pub fn is_valid_placement(grid_x: i32, grid_y: i32, arena_radius: f32) -> bool {
    let wx = grid_x as f32 * GRID_CELL_SIZE;
    let wy = grid_y as f32 * GRID_CELL_SIZE;
    let dist = (wx * wx + wy * wy).sqrt();
    dist + GRID_CELL_SIZE * 0.5 < arena_radius
}
