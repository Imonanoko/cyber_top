use serde::{Deserialize, Serialize};

use super::types::{MetersPerSec, Multiplier, Radius, SpinHp};

/// Pre-computed stats read during combat ticks. Read-only in FixedUpdate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveStats {
    pub spin_hp_max: SpinHp,
    pub radius: Radius,
    pub move_speed: MetersPerSec,
    /// Acceleration (world units per second squared).
    pub accel: f32,
    /// Control duration multiplier: m = max(0, 1 - R). Lower = more reduction.
    pub control_multiplier: f32,
    pub spin_drain_idle_per_sec: f32,
    pub spin_drain_on_wall_hit: f32,
    pub spin_drain_on_top_hit: f32,
    pub stability: f32,
    pub damage_out_mult: Multiplier,
    pub damage_in_mult: Multiplier,
    pub fire_rate_mult: Multiplier,
}

impl Default for EffectiveStats {
    fn default() -> Self {
        Self {
            spin_hp_max: SpinHp(100.0),
            radius: Radius(0.5),
            move_speed: MetersPerSec(5.0),
            accel: 25.0,
            control_multiplier: 1.0,
            spin_drain_idle_per_sec: 0.2,
            spin_drain_on_wall_hit: 0.5,
            spin_drain_on_top_hit: 1.0,
            stability: 0.0,
            damage_out_mult: Multiplier::one(),
            damage_in_mult: Multiplier::one(),
            fire_rate_mult: Multiplier::one(),
        }
    }
}
