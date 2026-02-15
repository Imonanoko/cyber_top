pub mod chassis;
pub mod shaft;
pub mod trait_screw;
pub mod weapon_wheel;

use serde::{Deserialize, Serialize};

use self::chassis::ChassisSpec;
use self::shaft::ShaftSpec;
use self::trait_screw::TraitScrewSpec;
use self::weapon_wheel::WeaponWheelSpec;
use crate::game::stats::modifier::ModifierSet;

/// A complete build: top + 4 parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Build {
    pub id: String,
    pub top_id: String,
    pub weapon: WeaponWheelSpec,
    pub shaft: ShaftSpec,
    pub chassis: ChassisSpec,
    pub screw: TraitScrewSpec,
    pub note: Option<String>,
}

impl Build {
    /// Combine all part modifiers into a single ModifierSet.
    pub fn combined_modifiers(&self) -> ModifierSet {
        let mut mods = ModifierSet::new();
        mods.merge(&self.shaft.to_modifiers());
        mods.merge(&self.chassis.to_modifiers());
        mods.merge(&self.screw.to_modifiers());
        mods
    }
}

impl Default for Build {
    fn default() -> Self {
        Self {
            id: "default_build".into(),
            top_id: "default_top".into(),
            weapon: WeaponWheelSpec::default(),
            shaft: ShaftSpec::default(),
            chassis: ChassisSpec::default(),
            screw: TraitScrewSpec::default(),
            note: None,
        }
    }
}
