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
    /// Additive bonus to acceleration.
    pub accel_add: f32,
    /// Multiplier to acceleration.
    pub accel_mul: f32,
    /// Additive bonus to collision radius.
    pub radius_add: f32,
    /// Multiplier to collision radius.
    pub radius_mul: f32,
}

impl Default for ChassisSpec {
    fn default() -> Self {
        Self {
            id: "default_chassis".into(),
            name: "Standard Chassis".into(),
            move_speed_add: 0.0,
            move_speed_mul: 1.0,
            accel_add: 0.0,
            accel_mul: 1.0,
            radius_add: 0.0,
            radius_mul: 1.0,
        }
    }
}

impl ChassisSpec {
    pub fn to_modifiers(&self) -> ModifierSet {
        let mut mods = ModifierSet::new();
        mods.move_speed.add = self.move_speed_add;
        mods.move_speed.mul = self.move_speed_mul;
        mods.accel.add = self.accel_add;
        mods.accel.mul = self.accel_mul;
        mods.radius.add = self.radius_add;
        mods.radius.mul = self.radius_mul;
        mods
    }
}
