use serde::{Deserialize, Serialize};

use super::types::{MetersPerSec, Radius, SpinHp};

/// Immutable base parameters for a Top.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseStats {
    pub id: String,
    pub name: String,
    pub spin_hp_max: SpinHp,
    pub radius: Radius,
    pub move_speed: MetersPerSec,
    /// Acceleration (world units per second squared).
    pub accel: f32,
    /// Control reduction ratio (positive = reduces control duration).
    pub control_reduction: f32,
}

impl Default for BaseStats {
    fn default() -> Self {
        Self {
            id: "default_top".into(),
            name: "Standard Top".into(),
            spin_hp_max: SpinHp(100.0),
            radius: Radius(1.3),
            move_speed: MetersPerSec(10.0),
            accel: 25.0,
            control_reduction: 0.0,
        }
    }
}
