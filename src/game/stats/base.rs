use serde::{Deserialize, Serialize};

use super::types::{MetersPerSec, Radius, SpinHp};

/// Immutable base parameters for a Top.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseStats {
    pub spin_hp_max: SpinHp,
    pub radius: Radius,
    pub move_speed: MetersPerSec,
    /// Control reduction ratio (positive = reduces control duration).
    pub control_reduction: f32,
}

impl Default for BaseStats {
    fn default() -> Self {
        Self {
            spin_hp_max: SpinHp(100.0),
            radius: Radius(1.2),
            move_speed: MetersPerSec(100.0),
            control_reduction: 0.0,
        }
    }
}
