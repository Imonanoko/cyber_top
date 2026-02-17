# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust + Bevy 0.18 desktop game: spinning tops battle in a frictionless circular arena. Two game modes: PvP and PvAI. Includes a Design Workshop for creating custom parts and builds.

## Build & Run

```bash
cargo build              # Build
cargo run                # Run the game
cargo build --release    # Release build
```

No test suite yet. No linter config beyond default `cargo` warnings.

## Key Documentation

- `docs/architecture.md` — Game flow, plugin structure, FixedUpdate pipeline, asset specs, Bevy 0.18 API notes
- `docs/design.md` — Game design spec: damage model, weapon system, equipment, numerical safety, DB schema

## Architecture Principles

1. **Event-driven resolution**: collisions/attacks produce events (`GameEvent`), never directly modify HP.
2. **Data-driven specs**: parts/tops/weapons defined in `PartRegistry` with ID-based lookup.
3. **Build-based selection**: players select complete builds (top + weapon + shaft + chassis + screw), not individual parts.
4. **No DB IO in combat tick**: only read in-memory `EffectiveStats` and runtime state.
5. **All tunable params in `Tuning` resource**: loaded from `tuning.ron`, F5 hot-reload at runtime.

## State Machine

```
MainMenu → Selection → PickMap / PickTop → Aiming → Battle → GameOver → MainMenu
                 ↕
          DesignHub → ManageParts → EditWeapon / EditShaft / EditChassis / EditScrew
                                 → AssembleBuild → PickDesignPart
```

All `GamePhase` states defined in `src/game/components.rs`. Menu UI in `src/plugins/menu_plugin.rs`. Game systems in `src/plugins/game_plugin.rs`. Design workshop in `src/plugins/design_plugin.rs`.

## FixedUpdate Pipeline (Battle phase only)

PhysicsSet → CollisionDetectSet → EventGenerateSet → HookProcessSet → EventApplySet → CleanupSet

All sets gated to `GamePhase::Battle` and chained in strict order. See `docs/architecture.md` for system-level detail.

## Bevy 0.18 API Gotchas

- `ChildSpawnerCommands` (not `ChildBuilder`) — the type for `with_children` closures
- `despawn()` is recursive by default — `despawn_recursive()` was removed
- `BorderRadius` is a field on `Node { border_radius: ..., ..default() }`, not a Component
- Bundle tuple max ~15 elements — nest inner tuples if needed
- Use `MessageWriter<T>` / `MessageReader<T>` for game events (not `Events<T>`)
- Use `MessageReader<KeyboardInput>` for keyboard events (not `EventReader<KeyboardInput>`)
- Query conflict (B0001): must use `Without<T>` to prove disjointness; `Changed<T>` does NOT help

## Entity Lifecycle

- Tag all game-session entities with `InGame` component
- `cleanup_game` on `OnEnter(MainMenu)` despawns all `InGame` entities
- Projectiles despawn on: hit, lifetime expiry, or leaving arena boundary

## Key Resources

| Resource | Location | Purpose |
|----------|----------|---------|
| `Tuning` | `src/config/tuning.rs` | All tunable constants |
| `PartRegistry` | `src/game/parts/registry.rs` | Data-driven parts, tops, weapons, builds (`BuildRef` entries) |
| `GameSelection` | `src/plugins/menu_plugin.rs` | Current mode, map, P1/P2 build IDs |
| `GameAssets` | `src/assets_map.rs` | Sprite handles + SFX handles, loaded at startup |
| `ProjectileAssets` | `src/game/components.rs` | Projectile mesh/material + per-weapon sprite handles |
| `DesignState` | `src/plugins/design_plugin.rs` | Design workshop state (editing part, build assembly slots) |
| `SqliteRepo` | `src/storage/sqlite_repo.rs` | SQLite persistence for parts and builds |
| `TokioRuntime` | `src/plugins/storage_plugin.rs` | Async-to-sync bridge for SQLx |

## Build System

Players select **builds** (not individual tops + weapons). A build = Top + Weapon + Shaft + Chassis + Screw.

- `BuildRef` in `PartRegistry.builds` stores part IDs
- `resolve_build()` assembles full `Build` struct from part IDs
- Default builds: `default_blade` (Melee), `default_blaster` (Ranged)
- Custom builds: created in Design Workshop, saved to SQLite, loaded at startup via `merge_custom_builds()`

## Asset System

Convention-based: part ID `"basic_blade"` → `assets/weapons/basic_blade.png`. Override via optional `sprite_path` field in specs.

- **Missing image** → procedural mesh fallback (game looks the same as without assets)
- **Missing audio** → silence (Bevy handles gracefully)
- Audio: one-shot `AudioPlayer` entities with `PlaybackSettings::DESPAWN`
- Sprites: `Sprite { image, custom_size }` for game entities, `ImageNode` for UI previews

### Image Specifications (PNG, RGBA)

| Asset Type | Recommended Size | Notes |
|------------|-----------------|-------|
| Top | 128x128 px | Facing right (+X). Scaled to world radius in-game |
| Weapon | 128x32 (melee) / 64x32 (ranged) | Facing right (+X). Width=length, Height=thickness |
| Projectile | 32x32 px | Scaled via Transform.scale in-game |
| Shaft / Chassis / Screw | 128x128 px | UI preview only, not rendered in-game |
| UI Icons | 32x32 px | Provide `{name}.png` + `{name}_hover.png` |

### Asset Directory Layout

```
assets/
  tops/           # {top_id}.png
  weapons/        # {weapon_id}.png
  projectiles/    # {weapon_id}_projectile.png
  shafts/         # {shaft_id}.png
  chassis/        # {chassis_id}.png
  screws/         # {screw_id}.png
  ui/             # edit.png, delete.png, + hover variants
  audio/sfx/      # launch.ogg, collision_top.ogg, etc.
```
