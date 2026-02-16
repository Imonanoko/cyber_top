use std::collections::HashMap;

use bevy::prelude::*;

use super::chassis::ChassisSpec;
use super::shaft::ShaftSpec;
use super::trait_screw::TraitScrewSpec;
use super::weapon_wheel::{MeleeSpec, RangedSpec, WeaponWheelSpec};
use super::Build;
use crate::game::stats::base::BaseStats;
use crate::game::stats::types::{MetersPerSec, Radius, SpinHp, WeaponKind};

/// Registry of all available parts and tops, indexed by ID.
/// Currently populated with hardcoded presets.
/// Future: load from DB tables (`tops`, `parts`).
#[derive(Resource, Default)]
pub struct PartRegistry {
    pub tops: HashMap<String, BaseStats>,
    pub weapons: HashMap<String, WeaponWheelSpec>,
    pub shafts: HashMap<String, ShaftSpec>,
    pub chassis: HashMap<String, ChassisSpec>,
    pub screws: HashMap<String, TraitScrewSpec>,
}

impl PartRegistry {
    /// Populate with hardcoded preset parts.
    pub fn with_defaults() -> Self {
        let mut reg = Self::default();

        // ── Tops ─────────────────────────────────────────────────
        reg.tops.insert("default_top".into(), BaseStats::default());
        reg.tops.insert(
            "small_top".into(),
            BaseStats {
                id: "small_top".into(),
                name: "Small Top".into(),
                spin_hp_max: SpinHp(80.0),
                radius: Radius(0.35),
                move_speed: MetersPerSec(6.0),
                accel: 30.0,
                control_reduction: 0.0,
                sprite_path: None,
            },
        );

        // ── Weapons ────────────────────────────────────────────────
        reg.weapons.insert(
            "basic_blade".into(),
            WeaponWheelSpec {
                id: "basic_blade".into(),
                name: "Basic Blade".into(),
                kind: WeaponKind::Melee,
                melee: Some(MeleeSpec::default()),
                ranged: None,
                sprite_path: None,
                projectile_sprite_path: None,
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
                sprite_path: None,
                projectile_sprite_path: None,
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
        let top = self.tops.get(top_id)?.clone();
        let weapon = self.weapons.get(weapon_id)?.clone();
        let shaft = self.shafts.get(shaft_id)?.clone();
        let chassis = self.chassis.get(chassis_id)?.clone();
        let screw = self.screws.get(screw_id)?.clone();

        Some(Build {
            id: build_id.into(),
            top,
            weapon,
            shaft,
            chassis,
            screw,
            note: None,
        })
    }
}
