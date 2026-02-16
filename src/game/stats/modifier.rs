use serde::{Deserialize, Serialize};

use super::base::BaseStats;
use super::effective::EffectiveStats;
use super::types::Multiplier;
use crate::config::tuning::Tuning;

/// A single stat modifier with add / mul / clamp.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatModifier {
    pub add: f32,
    pub mul: f32, // defaults to 1.0
    pub clamp_min: Option<f32>,
    pub clamp_max: Option<f32>,
}

impl StatModifier {
    pub fn identity() -> Self {
        Self {
            add: 0.0,
            mul: 1.0,
            clamp_min: None,
            clamp_max: None,
        }
    }

    pub fn apply(&self, base: f32) -> f32 {
        let v = (base + self.add) * self.mul;
        let v = if let Some(min) = self.clamp_min {
            v.max(min)
        } else {
            v
        };
        if let Some(max) = self.clamp_max {
            v.min(max)
        } else {
            v
        }
    }
}

/// Collection of modifiers for all stat fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModifierSet {
    pub spin_hp_max: StatModifier,
    pub radius: StatModifier,
    pub move_speed: StatModifier,
    pub accel: StatModifier,
    /// Each control_reduction source contributes a ratio r_i.
    pub control_reduction_sources: Vec<f32>,
    pub stability: StatModifier,
    pub spin_efficiency: StatModifier,
    pub damage_out_mult: Multiplier,
    pub damage_in_mult: Multiplier,
    pub fire_rate_mult: Multiplier,
}

impl ModifierSet {
    pub fn new() -> Self {
        Self {
            spin_hp_max: StatModifier::identity(),
            radius: StatModifier::identity(),
            move_speed: StatModifier::identity(),
            accel: StatModifier::identity(),
            control_reduction_sources: Vec::new(),
            stability: StatModifier::identity(),
            spin_efficiency: StatModifier::identity(),
            damage_out_mult: Multiplier::one(),
            damage_in_mult: Multiplier::one(),
            fire_rate_mult: Multiplier::one(),
        }
    }

    /// Merge another modifier set into this one.
    pub fn merge(&mut self, other: &ModifierSet) {
        self.spin_hp_max.add += other.spin_hp_max.add;
        self.spin_hp_max.mul *= other.spin_hp_max.mul;
        self.radius.add += other.radius.add;
        self.radius.mul *= other.radius.mul;
        self.move_speed.add += other.move_speed.add;
        self.move_speed.mul *= other.move_speed.mul;
        self.accel.add += other.accel.add;
        self.accel.mul *= other.accel.mul;
        self.control_reduction_sources
            .extend(&other.control_reduction_sources);
        self.stability.add += other.stability.add;
        self.stability.mul *= other.stability.mul;
        self.spin_efficiency.add += other.spin_efficiency.add;
        self.spin_efficiency.mul *= other.spin_efficiency.mul;
        self.damage_out_mult = self.damage_out_mult * other.damage_out_mult;
        self.damage_in_mult = self.damage_in_mult * other.damage_in_mult;
        self.fire_rate_mult = self.fire_rate_mult * other.fire_rate_mult;
    }

    /// Compute EffectiveStats from BaseStats + this modifier set + tuning.
    pub fn compute_effective(&self, base: &BaseStats, tuning: &Tuning) -> EffectiveStats {
        let spin_hp_max = self.spin_hp_max.apply(base.spin_hp_max.0).max(0.0);
        let radius = self.radius.apply(base.radius.0).max(0.01);
        let move_speed = self
            .move_speed
            .apply(base.move_speed.0)
            .clamp(0.0, tuning.max_speed);

        // Control reduction: multiplicative stacking
        // R = Π(1 + r_i) - 1; m = max(0, 1 - R)
        let mut combined = 1.0_f32;
        for &r in &self.control_reduction_sources {
            combined *= 1.0 + r;
        }
        combined *= 1.0 + base.control_reduction;
        let big_r = combined - 1.0;
        let control_multiplier = (1.0 - big_r).max(0.0);

        let accel = self.accel.apply(base.accel).max(0.0);

        let stability = self.stability.apply(0.0).max(0.0);
        let spin_efficiency = self.spin_efficiency.apply(1.0).clamp(0.0, 10.0);

        EffectiveStats {
            spin_hp_max: super::types::SpinHp(spin_hp_max),
            radius: super::types::Radius(radius),
            move_speed: super::types::MetersPerSec(move_speed),
            accel,
            control_multiplier,
            spin_drain_idle_per_sec: tuning.spin_drain_idle_per_sec / spin_efficiency,
            spin_drain_on_wall_hit: tuning.spin_drain_on_wall_hit,
            spin_drain_on_top_hit: tuning.spin_drain_on_top_hit,
            stability,
            damage_out_mult: self.damage_out_mult,
            damage_in_mult: self.damage_in_mult,
            fire_rate_mult: self.fire_rate_mult,
        }
    }
}
