use serde::{Deserialize, Serialize};

use crate::game::stats::modifier::ModifierSet;

/// Chassis specification: movement and acceleration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChassisSpec {
    pub id: String,
    pub name: String,
    /// Additive bonus to move speed.
    pub move_speed_add: f32,
    /// Multiplier to move speed.
    pub move_speed_mul: f32,
}

impl Default for ChassisSpec {
    fn default() -> Self {
        Self {
            id: "default_chassis".into(),
            name: "Standard Chassis".into(),
            move_speed_add: 0.0,
            move_speed_mul: 1.0,
        }
    }
}

impl ChassisSpec {
    pub fn to_modifiers(&self) -> ModifierSet {
        let mut mods = ModifierSet::new();
        mods.move_speed.add = self.move_speed_add;
        mods.move_speed.mul = self.move_speed_mul;
        mods
    }
}
