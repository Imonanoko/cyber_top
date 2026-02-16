# Cyber Top - Architecture

## Overview

Cyber Top is a 2D spinning-tops battle game built with **Rust + Bevy 0.18**.
Two tops are launched into a circular arena; physics (collisions, wall reflections) drives all movement after launch. The last top with spin HP > 0 wins.

---

## Game Flow (State Machine)

```
MainMenu → Selection → PickMap / PickTop → Aiming → Battle → GameOver → MainMenu
```

### GamePhase States
- **MainMenu**: Title screen with Start Game, Design Map (Coming Soon), Design Top (Coming Soon)
- **Selection**: Hub screen — choose mode (PvP / PvAI), map, P1/P2 tops+weapons
- **PickMap**: Dedicated map picker with card-based preview UI
- **PickTop**: Dedicated top+weapon picker with stats preview (reused for P1 and P2 via `PickingFor` resource)
- **Aiming**: Player rotates launch direction (Arrow keys + Space). P2: A/D + Enter. AI auto-confirms random angle.
- **Battle**: Physics-driven combat. FixedUpdate systems run.
- **GameOver**: Winner overlay. ESC/Enter returns to MainMenu.

### Game Modes
- **PvAI**: Player vs AI. AI randomly selects top+weapon from pool.
- **PvP**: Player vs Player. Both players pick loadout and aim manually.

---

## Plugin Architecture

| Plugin | File | Role |
|--------|------|------|
| `GamePlugin` | `plugins/game_plugin.rs` | FixedUpdate scheduling, arena setup, aiming, launch, game over |
| `MenuPlugin` | `plugins/menu_plugin.rs` | MainMenu, Selection hub, Map/Top pickers, GameOver overlay |
| `UiPlugin` | `plugins/ui_plugin.rs` | In-game HUD (HP, velocity, phase text) |
| `StoragePlugin` | `plugins/storage_plugin.rs` | SQLite/SQLx init |

---

## FixedUpdate Pipeline (Battle phase only)

SystemSets in strict chain order:

```
1. PhysicsSet (chained):
   integrate_physics → integrate_projectiles → spin_drain →
   tick_control_state → tick_status_effects → tick_melee_trackers →
   wall_reflection

2. CollisionDetectSet:
   detect_collisions

3. EventGenerateSet (chained):
   generate_collision_damage → detect_melee_hits → fire_ranged_weapons

4. HookProcessSet:
   process_hooks (v0 no-op)

5. EventApplySet (chained):
   apply_damage_events → apply_control_events → resolve_top_collisions →
   spawn_obstacles → spawn_projectiles

6. CleanupSet (chained):
   despawn_projectiles_outside_arena → cleanup_ttl → handle_despawn_events
```

---

## Key Resources

| Resource | Description |
|----------|-------------|
| `Tuning` | All tunable params, loaded from `tuning.ron`, F5 hot-reload |
| `PartRegistry` | Data-driven part presets (tops, weapons, shafts, etc.) |
| `GameSelection` | Current mode, map, P1/P2 top and weapon IDs |
| `PickingFor` | Which player (1 or 2) is in the picker screen |
| `ProjectileAssets` | Pre-built mesh/material for projectile rendering |
| `AssetsMap` | Skin color lookup (placeholder for sprites) |

---

## Entity Lifecycle

- **InGame marker**: All game-session entities tagged with `InGame` component
- **Cleanup**: `cleanup_game` on `OnEnter(MainMenu)` despawns all `InGame` entities
- **Projectiles**: Despawned on hit, lifetime expiry, or leaving arena boundary
- **Obstacles**: Despawned when `ExpiresAt` time is reached

---

## Message System (Bevy B0002 workaround)

- `CollisionMessage`: Top-Top collision data (separate type to avoid Res/ResMut conflict)
- `GameEvent`: DealDamage, ApplyControl, ApplyStatus, SpawnProjectile, SpawnObstacle, DespawnEntity

---

## Stats Architecture (3-Layer)

1. `BaseStats` — immutable base parameters per top
2. `ModifierSet` — from parts + passive traits + status
3. `EffectiveStats` — Base + modifiers applied; cached, recomputed on loadout change

---

## Key Design Decisions

- **Coordinate system**: All positions/sizes in world units. Camera `OrthographicProjection` with `scale = 1/pixels_per_unit` handles zoom.
- **Elastic collisions**: `wall_bounce_damping = 1.0` and `top_collisions_restitution = 1.0` by default.
- **Weapon visuals**: Spawned as child entities of tops. Parent rotation auto-rotates children.
- **Projectile visuals**: Unit circle mesh scaled per-projectile via `Transform.scale`.
- **Data-driven parts**: `PartRegistry` holds all parts by ID. `setup_arena()` uses `resolve_build()` to assemble builds from IDs.
- **Initial aim direction**: Each top starts aimed toward the opponent (P1: angle 0, P2: angle PI).

---

## Bevy 0.18 API Notes

- `ChildSpawnerCommands` (not `ChildBuilder`) for `with_children` closure parameter
- `despawn()` is recursive by default (`despawn_recursive()` was removed)
- `BorderRadius` is a field on `Node`, not a standalone Component
- Bundle tuple max size ~15 elements — nest tuples if needed
- Use `MessageWriter`/`MessageReader` for game events (not `Events`)
