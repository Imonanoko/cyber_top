use std::collections::HashMap;

use bevy::prelude::*;

use super::chassis::ChassisSpec;
use super::shaft::ShaftSpec;
use super::trait_screw::TraitScrewSpec;
use super::weapon_wheel::{MeleeSpec, RangedSpec, WeaponWheelSpec};
use super::Build;
use crate::game::stats::types::WeaponKind;

/// Registry of all available parts, indexed by ID.
/// Currently populated with hardcoded presets.
/// Future: load from `parts` DB table (`id, slot, kind, spec_json`).
#[derive(Resource, Default)]
pub struct PartRegistry {
    pub weapons: HashMap<String, WeaponWheelSpec>,
    pub shafts: HashMap<String, ShaftSpec>,
    pub chassis: HashMap<String, ChassisSpec>,
    pub screws: HashMap<String, TraitScrewSpec>,
}

impl PartRegistry {
    /// Populate with hardcoded preset parts.
    pub fn with_defaults() -> Self {
        let mut reg = Self::default();

        // ── Weapons ────────────────────────────────────────────────
        reg.weapons.insert(
            "basic_blade".into(),
            WeaponWheelSpec {
                id: "basic_blade".into(),
                name: "Basic Blade".into(),
                kind: WeaponKind::Melee,
                melee: Some(MeleeSpec::default()),
                ranged: None,
            },
        );

        reg.weapons.insert(
            "basic_blaster".into(),
            WeaponWheelSpec {
                id: "basic_blaster".into(),
                name: "Basic Blaster".into(),
                kind: WeaponKind::Ranged,
                melee: None,
                ranged: Some(RangedSpec::default()),
            },
        );

        // ── Shafts ─────────────────────────────────────────────────
        reg.shafts
            .insert("standard_shaft".into(), ShaftSpec::default());

        // ── Chassis ────────────────────────────────────────────────
        reg.chassis
            .insert("standard_chassis".into(), ChassisSpec::default());

        // ── Trait Screws ───────────────────────────────────────────
        reg.screws
            .insert("standard_screw".into(), TraitScrewSpec::default());

        reg
    }

    /// Assemble a `Build` by looking up each part ID in the registry.
    /// Returns `None` if any part ID is not found.
    pub fn resolve_build(
        &self,
        build_id: &str,
        top_id: &str,
        weapon_id: &str,
        shaft_id: &str,
        chassis_id: &str,
        screw_id: &str,
    ) -> Option<Build> {
        let weapon = self.weapons.get(weapon_id)?.clone();
        let shaft = self.shafts.get(shaft_id)?.clone();
        let chassis = self.chassis.get(chassis_id)?.clone();
        let screw = self.screws.get(screw_id)?.clone();

        Some(Build {
            id: build_id.into(),
            top_id: top_id.into(),
            weapon,
            shaft,
            chassis,
            screw,
            note: None,
        })
    }
}
