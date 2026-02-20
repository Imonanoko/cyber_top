# Data Model — Parts, Builds, Stats, Storage

> All data structures for the game's part/build system and persistence layer.

---

## Part Slots

```rust
// src/game/stats/types.rs
pub enum PartSlot {
    WeaponWheel,
    Shaft,
    Chassis,
    TraitScrew,
}
```

Note: Top body is **not** a PartSlot — it's a separate concept (`BaseStats`).

---

## Part Specs (per slot)

### Top Body — `BaseStats` (`game/stats/base.rs`)

```rust
pub struct BaseStats {
    pub id: String,
    pub name: String,
    pub spin_hp_max: SpinHp,        // Default 100.0
    pub radius: Radius,              // Default 1.3
    pub move_speed: MetersPerSec,    // Default 10.0
    pub accel: f32,                  // Default 25.0
    pub control_reduction: f32,      // Default 0.0
    pub sprite_path: Option<String>, // Override convention-based path
}
```

### Weapon — `WeaponWheelSpec` (`game/parts/weapon_wheel.rs`)

```rust
pub struct WeaponWheelSpec {
    pub id: String,
    pub name: String,
    pub kind: WeaponKind,            // Melee or Ranged
    pub melee: Option<MeleeSpec>,    // Populated when kind=Melee
    pub ranged: Option<RangedSpec>,  // Populated when kind=Ranged
    pub sprite_path: Option<String>,
    pub projectile_sprite_path: Option<String>,
}

pub enum WeaponKind { Melee, Ranged }

pub struct MeleeSpec {
    pub base_damage: f32,
    pub hit_cooldown: f32,
    pub max_hits_per_rotation: u32,
    pub hitbox_radius: f32,
    pub hitbox_angle: f32,           // Radians, default PI/3
    pub hit_control: Option<ControlEffect>,
    pub spin_rate_multiplier: f32,
    pub blade_len: f32,
    pub blade_thick: f32,
}

pub struct RangedSpec {
    pub projectile_damage: f32,
    pub fire_rate: f32,              // Shots/sec
    pub burst_count: u32,
    pub spread_angle: f32,           // Radians
    pub knockback_distance: f32,
    pub projectile_radius: f32,
    pub control_duration: Seconds,
    pub lifetime: Seconds,
    pub projectile_speed: f32,
    pub aim_mode: AimMode,
    pub spin_rate_multiplier: f32,
    pub barrel_len: f32,
    pub barrel_thick: f32,
}
```

### Shaft — `ShaftSpec` (`game/parts/shaft.rs`)

```rust
pub struct ShaftSpec {
    pub id: String,
    pub name: String,
    pub stability: f32,          // Reduces collision displacement
    pub spin_efficiency: f32,    // Reduces idle spin drain (multiplier)
}
```

### Chassis — `ChassisSpec` (`game/parts/chassis.rs`)

```rust
pub struct ChassisSpec {
    pub id: String,
    pub name: String,
    pub move_speed_add: f32,     // Flat speed bonus
    pub move_speed_mul: f32,     // Speed multiplier (1.0 = unchanged)
    pub accel_add: f32,
    pub accel_mul: f32,
    pub radius_add: f32,         // Collision radius bonus
    pub radius_mul: f32,
}
```

### Trait Screw — `TraitScrewSpec` (`game/parts/trait_screw.rs`)

```rust
pub struct TraitScrewSpec {
    pub id: String,
    pub name: String,
    pub passive: TraitPassive,
    pub hooks: Vec<TraitHookKind>,  // Future: event hooks
}

pub struct TraitPassive {
    pub spin_hp_max_add: f32,       // Max HP bonus
    pub control_reduction: f32,     // Added to control_reduction sources
    pub damage_out_mult: f32,       // Outgoing damage multiplier
    pub damage_in_mult: f32,        // Incoming damage multiplier
}
```

---

## Build System

### BuildRef (lightweight, in-memory) — `game/parts/registry.rs`

```rust
pub struct BuildRef {
    pub id: String,
    pub name: String,
    pub top_id: String,
    pub weapon_id: String,
    pub shaft_id: String,
    pub chassis_id: String,
    pub screw_id: String,
}
```

### Build (resolved, full specs) — `game/parts/mod.rs`

```rust
pub struct Build {
    pub id: String,
    pub name: String,
    pub top: BaseStats,
    pub weapon: WeaponWheelSpec,
    pub shaft: ShaftSpec,
    pub chassis: ChassisSpec,
    pub screw: TraitScrewSpec,
    pub note: Option<String>,
}
```

### Resolution Flow

```
BuildRef (IDs only)
  → PartRegistry.resolve_build(build_id, build_name, top_id, weapon_id, shaft_id, chassis_id, screw_id)
  → Build (full specs)
  → Build.combined_modifiers() → ModifierSet
  → ModifierSet.compute_effective(base, tuning) → EffectiveStats
```

---

## PartRegistry — `game/parts/registry.rs`

```rust
pub struct PartRegistry {
    pub tops: HashMap<String, BaseStats>,
    pub weapons: HashMap<String, WeaponWheelSpec>,
    pub shafts: HashMap<String, ShaftSpec>,
    pub chassis: HashMap<String, ChassisSpec>,
    pub screws: HashMap<String, TraitScrewSpec>,
    pub builds: HashMap<String, BuildRef>,
}
```

### Lifecycle

1. `PartRegistry::with_defaults()` — populates hardcoded presets
2. `merge_custom_parts(repo, rt)` — loads from SQLite `parts` table (all slots + tops)
3. `merge_custom_builds(repo, rt)` — loads from SQLite `builds` table
4. At runtime: editors save to SQLite AND insert into the HashMap immediately

### Default Parts

| ID | Type |
|----|------|
| `default_top` | Top body |
| `basic_blade` | Weapon (Melee) |
| `basic_blaster` | Weapon (Ranged) |
| `standard_shaft` | Shaft |
| `standard_chassis` | Chassis |
| `standard_screw` | Screw |

### Default Builds

| ID | Name | Composition |
|----|------|-------------|
| `default_blade` | Standard Blade Top | default_top + basic_blade + standard_* |
| `default_blaster` | Standard Blaster Top | default_top + basic_blaster + standard_* |

---

## Stats Architecture (3-Layer)

### Layer 1: BaseStats
Immutable parameters per top body. Stored in `PartRegistry.tops`.

### Layer 2: ModifierSet (`game/stats/modifier.rs`)

```rust
pub struct StatModifier {
    pub add: f32,       // Additive bonus
    pub mul: f32,       // Multiplicative (default 1.0)
    pub clamp_min: Option<f32>,
    pub clamp_max: Option<f32>,
}

pub struct ModifierSet {
    pub spin_hp_max: StatModifier,
    pub radius: StatModifier,
    pub move_speed: StatModifier,
    pub accel: StatModifier,
    pub control_reduction_sources: Vec<f32>,  // Multiplicative stacking
    pub stability: StatModifier,
    pub spin_efficiency: StatModifier,
    pub damage_out_mult: Multiplier,
    pub damage_in_mult: Multiplier,
    pub fire_rate_mult: Multiplier,
}
```

- `merge(&mut self, other)` stacks modifiers from multiple parts
- `compute_effective(base, tuning)` produces final `EffectiveStats`
- Control reduction: `R = product(1 + r_i) - 1`, multiplier = `max(0, 1 - R)`

### Layer 3: EffectiveStats (`game/stats/effective.rs`)

Read-only computed stats used during battle. Cached per build.

---

## SQLite Persistence — `storage/sqlite_repo.rs`

### Tables

| Table | Columns | Purpose |
|-------|---------|---------|
| `parts` | `id, slot, kind, spec_json` | All custom parts (JSON blob) |
| `builds` | `id, top_id, weapon_id, shaft_id, chassis_id, screw_id, note` | Custom builds |
| `effective_cache` | `build_id, effective_stats_json, computed_at, balance_version, hash` | Stats cache |

### Key Sync Methods (used by design plugin)

```rust
// Parts
repo.save_part_sync(rt, slot, kind, id, spec_json) -> Result<(), String>
repo.load_parts_by_slot_sync(rt, slot) -> Result<Vec<(id, kind, json)>, String>
repo.delete_part_sync(rt, id) -> Result<(), String>

// Builds
repo.save_build_sync(rt, build: &Build) -> Result<(), String>
repo.load_all_builds_sync(rt) -> Result<Vec<(id, top_id, weapon_id, shaft_id, chassis_id, screw_id, note)>, String>
repo.delete_build_sync(rt, id) -> Result<(), String>
```

### Save Patterns

Parts are saved as JSON via `serde_json::to_string(&spec)`:
- Top: `save_part_sync(rt, "top", "top", &id, &json)`
- Weapon: `save_part_sync(rt, "weapon", &kind_str, &id, &json)`
- Shaft: `save_part_sync(rt, "shaft", "shaft", &id, &json)`
- Chassis: `save_part_sync(rt, "chassis", "chassis", &id, &json)`
- Screw: `save_part_sync(rt, "screw", "screw", &id, &json)`

Builds are saved via `save_build_sync(rt, &build)` which writes to the `builds` table.

---

## Asset Conventions

Part ID determines file path automatically:

| Part Type | Asset Path |
|-----------|-----------|
| Top | `assets/tops/{id}.png` |
| Weapon | `assets/weapons/{id}.png` |
| Projectile | `assets/projectiles/{id}_projectile.png` |
| Shaft | `assets/shafts/{id}.png` |
| Chassis | `assets/chassis/{id}.png` |
| Screw | `assets/screws/{id}.png` |

Missing images → procedural fallback mesh (game still works).

The design workshop's "Set Image" button uses `rfd::FileDialog` to pick a PNG, which is copied to the correct `assets/{slot}/` directory using the part's pre-generated ID.
