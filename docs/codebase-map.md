# Codebase Map

> Quick-reference for locating code. Each entry = file path + what it contains.

---

## Directory Tree

```
src/
├── main.rs                          # App entry, window 1200x900, plugin registration
├── assets_map.rs                    # GameAssets resource (sprite + SFX handles)
├── config/
│   └── tuning.rs                    # Tuning resource, F5 hot-reload, tuning.ron
├── game/
│   ├── components.rs                # GamePhase enum, Top/Projectile markers, runtime state
│   ├── events.rs                    # GameEvent, CollisionMessage (Message types)
│   ├── collision.rs                 # detect_collisions (top-top, top-wall, projectile-top)
│   ├── combat.rs                    # Damage/control apply, melee detect, ranged fire
│   ├── physics.rs                   # Integrate, spin drain, tick control/status/melee
│   ├── hooks.rs                     # Trait screw hook pipeline (v0: no-op)
│   ├── parts/
│   │   ├── mod.rs                   # Build struct (resolved top+weapon+parts)
│   │   ├── registry.rs              # PartRegistry, BuildRef, resolve_build()
│   │   ├── weapon_wheel.rs          # WeaponWheelSpec, MeleeSpec, RangedSpec
│   │   ├── shaft.rs                 # ShaftSpec (stability, spin_efficiency)
│   │   ├── chassis.rs               # ChassisSpec (speed/accel/radius mods)
│   │   └── trait_screw.rs           # TraitScrewSpec, TraitPassive, hooks
│   ├── stats/
│   │   ├── types.rs                 # Newtypes (SpinHp, Radius, etc.), enums (WeaponKind, PartSlot, ControlEffect)
│   │   ├── base.rs                  # BaseStats (immutable top params)
│   │   ├── effective.rs             # EffectiveStats (computed from base + mods)
│   │   └── modifier.rs             # StatModifier, ModifierSet, stacking logic
│   ├── status/
│   │   └── effect.rs                # StatusEffectDef, StatusEffectType
│   └── arena/
│       ├── circle.rs                # Wall reflection
│       └── obstacle.rs              # Obstacle collision + despawn
├── storage/
│   ├── repo.rs                      # BuildRepository trait (unused interface)
│   └── sqlite_repo.rs              # SqliteRepo: async+sync CRUD for parts/builds
└── plugins/
    ├── game_plugin.rs               # FixedUpdate pipeline, arena setup, aiming, launch
    ├── menu_plugin.rs               # MainMenu, Selection, MapPicker, BuildPicker
    ├── design_plugin.rs             # Design Workshop (all editors, manage, assembly)
    ├── storage_plugin.rs            # StoragePlugin, TokioRuntime resource
    └── ui_plugin.rs                 # Battle HUD (HP bars, phase text)
```

---

## Plugin → GamePhase Ownership

| Plugin | Owns These Phases |
|--------|-------------------|
| `MenuPlugin` | MainMenu, Selection, PickMap, PickTop, GameOver |
| `GamePlugin` | Aiming, Battle |
| `DesignPlugin` | DesignHub, EditTop, EditWeapon, EditShaft, EditChassis, EditScrew, ManageParts, AssembleBuild, PickDesignPart |

---

## Key Resources (where to find them)

| Resource | File | Purpose |
|----------|------|---------|
| `Tuning` | `config/tuning.rs` | All gameplay constants, hot-reloadable |
| `PartRegistry` | `game/parts/registry.rs` | All parts + builds in memory |
| `GameSelection` | `plugins/menu_plugin.rs` | Current mode, map, P1/P2 build IDs |
| `PickingFor` | `plugins/menu_plugin.rs` | Which player is in picker (1 or 2) |
| `DesignState` | `plugins/design_plugin.rs` | Workshop state (editing ID, build slots, errors) |
| `GameAssets` | `assets_map.rs` | Sprite + SFX handles |
| `ProjectileAssets` | `game/components.rs` | Projectile mesh/material/sprites |
| `SqliteRepo` | `storage/sqlite_repo.rs` | DB access |
| `TokioRuntime` | `plugins/storage_plugin.rs` | Async bridge |
