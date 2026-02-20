use std::collections::HashMap;

use bevy::prelude::*;

use super::chassis::ChassisSpec;
use super::shaft::ShaftSpec;
use super::trait_screw::TraitScrewSpec;
use super::weapon_wheel::{MeleeSpec, RangedSpec, WeaponWheelSpec};
use super::Build;
use crate::game::map::MapSpec;
use crate::game::stats::base::BaseStats;
use crate::game::stats::types::WeaponKind;

/// Lightweight reference to a build (stores part IDs, not resolved specs).
#[derive(Clone, Debug)]
pub struct BuildRef {
    pub id: String,
    pub name: String,
    pub top_id: String,
    pub weapon_id: String,
    pub shaft_id: String,
    pub chassis_id: String,
    pub screw_id: String,
}

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
    pub builds: HashMap<String, BuildRef>,
    pub maps: HashMap<String, MapSpec>,
}

impl PartRegistry {
    /// Populate with hardcoded preset parts.
    pub fn with_defaults() -> Self {
        let mut reg = Self::default();

        // ── Tops ─────────────────────────────────────────────────
        reg.tops.insert("default_top".into(), BaseStats::default());

        // ── Weapons ────────────────────────────────────────────────
        reg.weapons.insert(
            "basic_blade".into(),
            WeaponWheelSpec {
                id: "basic_blade".into(),
                name: "Standard Blade".into(),
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
                name: "Standard Blaster".into(),
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

        // ── Default Builds ───────────────────────────────────────
        reg.builds.insert(
            "default_blade".into(),
            BuildRef {
                id: "default_blade".into(),
                name: "Standard Blade Top".into(),
                top_id: "default_top".into(),
                weapon_id: "basic_blade".into(),
                shaft_id: "standard_shaft".into(),
                chassis_id: "standard_chassis".into(),
                screw_id: "standard_screw".into(),
            },
        );
        reg.builds.insert(
            "default_blaster".into(),
            BuildRef {
                id: "default_blaster".into(),
                name: "Standard Blaster Top".into(),
                top_id: "default_top".into(),
                weapon_id: "basic_blaster".into(),
                shaft_id: "standard_shaft".into(),
                chassis_id: "standard_chassis".into(),
                screw_id: "standard_screw".into(),
            },
        );

        // ── Default Maps ─────────────────────────────────────────────
        let default_map = MapSpec::default_arena();
        reg.maps.insert(default_map.id.clone(), default_map);

        reg
    }

    /// Load custom user-created parts from SQLite into the registry.
    pub fn merge_custom_parts(
        &mut self,
        repo: &crate::storage::sqlite_repo::SqliteRepo,
        rt: &tokio::runtime::Runtime,
    ) {
        if let Ok(parts) = repo.load_parts_by_slot_sync(rt, "top") {
            for (id, _kind, json) in parts {
                if let Ok(spec) = serde_json::from_str::<BaseStats>(&json) {
                    self.tops.insert(id, spec);
                }
            }
        }
        if let Ok(parts) = repo.load_parts_by_slot_sync(rt, "weapon") {
            for (id, _kind, json) in parts {
                if let Ok(spec) = serde_json::from_str::<WeaponWheelSpec>(&json) {
                    self.weapons.insert(id, spec);
                }
            }
        }
        if let Ok(parts) = repo.load_parts_by_slot_sync(rt, "shaft") {
            for (id, _kind, json) in parts {
                if let Ok(spec) = serde_json::from_str::<ShaftSpec>(&json) {
                    self.shafts.insert(id, spec);
                }
            }
        }
        if let Ok(parts) = repo.load_parts_by_slot_sync(rt, "chassis") {
            for (id, _kind, json) in parts {
                if let Ok(spec) = serde_json::from_str::<ChassisSpec>(&json) {
                    self.chassis.insert(id, spec);
                }
            }
        }
        if let Ok(parts) = repo.load_parts_by_slot_sync(rt, "screw") {
            for (id, _kind, json) in parts {
                if let Ok(spec) = serde_json::from_str::<TraitScrewSpec>(&json) {
                    self.screws.insert(id, spec);
                }
            }
        }
    }

    /// Load custom user-created builds from SQLite into the registry.
    pub fn merge_custom_builds(
        &mut self,
        repo: &crate::storage::sqlite_repo::SqliteRepo,
        rt: &tokio::runtime::Runtime,
    ) {
        if let Ok(rows) = repo.load_all_builds_sync(rt) {
            for (id, top_id, weapon_id, shaft_id, chassis_id, screw_id, note) in rows {
                let name = if note.is_empty() { id.clone() } else { note };
                self.builds.insert(
                    id.clone(),
                    BuildRef { id, name, top_id, weapon_id, shaft_id, chassis_id, screw_id },
                );
            }
        }
    }

    /// Load custom user-created maps from SQLite into the registry.
    pub fn merge_custom_maps(
        &mut self,
        repo: &crate::storage::sqlite_repo::SqliteRepo,
        rt: &tokio::runtime::Runtime,
    ) {
        if let Ok(rows) = repo.load_all_maps_sync(rt) {
            for (id, name, arena_radius, placements_json) in rows {
                let placements: Vec<crate::game::map::MapPlacement> =
                    serde_json::from_str(&placements_json).unwrap_or_default();
                self.maps.insert(
                    id.clone(),
                    MapSpec {
                        id,
                        name,
                        arena_radius: arena_radius as f32,
                        placements,
                    },
                );
            }
        }
    }

    /// Assemble a `Build` by looking up each part ID in the registry.
    /// Returns `None` if any part ID is not found.
    pub fn resolve_build(
        &self,
        build_id: &str,
        build_name: &str,
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
            name: build_name.into(),
            top,
            weapon,
            shaft,
            chassis,
            screw,
            note: None,
        })
    }
}
