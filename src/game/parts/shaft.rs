use serde::{Deserialize, Serialize};

use crate::game::stats::modifier::ModifierSet;

/// Shaft specification: stability + spin efficiency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaftSpec {
    pub id: String,
    pub name: String,
    /// Reduces collision displacement / knockback.
    pub stability: f32,
    /// Multiplier for idle spin drain (higher = less drain).
    pub spin_efficiency: f32,
}

impl Default for ShaftSpec {
    fn default() -> Self {
        Self {
            id: "default_shaft".into(),
            name: "Standard Shaft".into(),
            stability: 0.5,
            spin_efficiency: 1.0,
        }
    }
}

impl ShaftSpec {
    pub fn to_modifiers(&self) -> ModifierSet {
        let mut mods = ModifierSet::new();
        mods.stability.add = self.stability;
        mods.spin_efficiency.mul = self.spin_efficiency;
        mods
    }
}
