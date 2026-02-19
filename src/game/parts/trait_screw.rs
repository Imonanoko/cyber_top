use serde::{Deserialize, Serialize};

use crate::game::stats::modifier::ModifierSet;
use crate::game::stats::types::Multiplier;

/// Which events a trait screw can hook into.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraitHookKind {
    OnHit,
    OnTick,
    OnWallCollision,
    OnFireProjectile,
}

/// Passive stat changes from a trait screw.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitPassive {
    /// Additive spin HP max bonus.
    pub spin_hp_max_add: f32,
    /// Control reduction ratio added to sources.
    pub control_reduction: f32,
    /// Damage output multiplier.
    pub damage_out_mult: f32,
    /// Damage intake multiplier.
    pub damage_in_mult: f32,
}

impl Default for TraitPassive {
    fn default() -> Self {
        Self {
            spin_hp_max_add: 0.0,
            control_reduction: 0.0,
            damage_out_mult: 1.0,
            damage_in_mult: 1.0,
        }
    }
}

/// Trait screw specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitScrewSpec {
    pub id: String,
    pub name: String,
    pub passive: TraitPassive,
    pub hooks: Vec<TraitHookKind>,
}

impl Default for TraitScrewSpec {
    fn default() -> Self {
        Self {
            id: "default_screw".into(),
            name: "Standard Screw".into(),
            passive: TraitPassive::default(),
            hooks: Vec::new(),
        }
    }
}

impl TraitScrewSpec {
    pub fn to_modifiers(&self) -> ModifierSet {
        let mut mods = ModifierSet::new();
        mods.spin_hp_max.add = self.passive.spin_hp_max_add;
        if self.passive.control_reduction != 0.0 {
            mods.control_reduction_sources
                .push(self.passive.control_reduction);
        }
        mods.damage_out_mult = Multiplier::new(self.passive.damage_out_mult);
        mods.damage_in_mult = Multiplier::new(self.passive.damage_in_mult);
        mods
    }
}
