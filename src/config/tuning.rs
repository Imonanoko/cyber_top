use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// All tunable game parameters, loaded from tuning.ron.
#[derive(Debug, Clone, Resource, Serialize, Deserialize)]
pub struct Tuning {
    pub dt: f32,
    pub pixels_per_unit: f32,
    pub aim_arrow_len_px: f32,
    pub aim_arrow_thickness_px: f32,
    pub aim_arrow_offset_px: f32,
    pub arena_radius: f32,
    pub wall_bounce_damping: f32,
    pub top_collisions_restitution: f32,
    pub spin_drain_idle_per_sec: f32,
    pub spin_drain_on_wall_hit: f32,
    pub spin_drain_on_top_hit: f32,
    pub collision_damage_k: f32,
    pub wall_damage_k: f32,
    pub size_damage_k: f32,
    pub size_radius_ref: f32,
    pub max_speed: f32,
    pub input_accel: f32,
    /// Optional: melee hit_speed_scale coefficient. 0 = disabled.
    pub melee_speed_scale_k: f32,
    /// Default obstacle contact damage.
    pub obstacle_damage: f32,
    /// Aim rotation speed (radians per second).
    pub aim_speed: f32,
    /// Visual spin rate multiplier (velocity → visual rotation speed).
    pub spin_visual_k: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            pixels_per_unit: 25.0,
            aim_arrow_len_px: 60.0,
            aim_arrow_thickness_px: 4.0,
            aim_arrow_offset_px: 40.0,
            arena_radius: 12.0,
            wall_bounce_damping: 1.0,
            top_collisions_restitution: 1.0,
            spin_drain_idle_per_sec: 0.2,
            spin_drain_on_wall_hit: 0.5,
            spin_drain_on_top_hit: 1.0,
            collision_damage_k: 0.5,
            wall_damage_k: 0.3,
            size_damage_k: 0.0,
            size_radius_ref: 1.0,
            max_speed: 30.0,
            input_accel: 25.0,
            melee_speed_scale_k: 0.0,
            obstacle_damage: 2.0,
            aim_speed: 3.0,
            spin_visual_k: 2.0,
        }
    }
}

impl Tuning {
    /// Get the data directory for tuning files.
    pub fn data_dir() -> PathBuf {
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("cyber_top")
    }

    /// Path to the tuning file.
    pub fn file_path() -> PathBuf {
        Self::data_dir().join("tuning.ron")
    }

    /// Load from file, or create default if not found.
    pub fn load_or_default() -> Self {
        let path = Self::file_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match ron::from_str(&contents) {
                    Ok(tuning) => return tuning,
                    Err(e) => {
                        warn!("Failed to parse tuning.ron: {e}, using defaults");
                    }
                },
                Err(e) => {
                    warn!("Failed to read tuning.ron: {e}, using defaults");
                }
            }
        }
        let tuning = Self::default();
        tuning.save();
        tuning
    }

    /// Save current tuning to file.
    pub fn save(&self) {
        let path = Self::file_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let pretty = ron::ser::PrettyConfig::default();
        match ron::ser::to_string_pretty(self, pretty) {
            Ok(s) => {
                if let Err(e) = std::fs::write(&path, s) {
                    warn!("Failed to write tuning.ron: {e}");
                }
            }
            Err(e) => {
                warn!("Failed to serialize tuning: {e}");
            }
        }
    }

    /// Reload from file (called by key press).
    pub fn reload(&mut self) {
        *self = Self::load_or_default();
        info!("Tuning reloaded");
    }
}
