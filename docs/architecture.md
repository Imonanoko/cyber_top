# Cyber Top - Architecture

## Overview

Cyber Top is a 2D spinning-tops battle game built with **Rust + Bevy 0.18**.
Two tops are launched into a circular arena; physics (collisions, wall reflections) drives all movement after launch. The last top with spin HP > 0 wins.

---

## Game Flow (State Machine)

```
MainMenu → Selection → PickMap / PickTop → Aiming → Battle → GameOver → MainMenu
                 ↕
          DesignHub → ManageParts → EditTop / EditWeapon / EditShaft / EditChassis / EditScrew
                                 → AssembleBuild → PickDesignPart
```

### GamePhase States

**Game flow:**
- **MainMenu**: Title screen with Start Game, Design Map (Coming Soon), Design Top
- **Selection**: Hub screen — choose mode (PvP / PvAI), map, P1/P2 builds
- **PickMap**: Dedicated map picker with card-based preview UI
- **PickTop**: Build picker — select a complete build (top + weapon + parts). Reused for P1 and P2 via `PickingFor` resource.
- **Aiming**: Player rotates launch direction (Arrow keys + Space). P2: A/D + Enter. AI auto-confirms random angle.
- **Battle**: Physics-driven combat. FixedUpdate systems run.
- **GameOver**: Winner overlay. ESC/Enter returns to MainMenu.

**Design workshop flow:**
- **DesignHub**: Entry point — Create Part, Manage Parts
- **ManageParts**: List all custom parts and builds, edit/delete
- **EditTop**: Top body editor (spin HP, radius, speed, accel, control reduction)
- **EditWeapon / EditShaft / EditChassis / EditScrew**: Part editors with text inputs, image assignment, kind selector (weapon)
- **AssembleBuild**: Assemble a build by picking parts for each slot (top, weapon, shaft, chassis, screw)
- **PickDesignPart**: Pick a part for a specific slot during build assembly

### Game Modes
- **PvAI**: Player vs AI. AI randomly selects a build from available builds.
- **PvP**: Player vs Player. Both players pick a build and aim manually.

---

## Plugin Architecture

| Plugin | File | Role |
|--------|------|------|
| `GamePlugin` | `plugins/game_plugin.rs` | FixedUpdate scheduling, arena setup, aiming, launch, game over |
| `MenuPlugin` | `plugins/menu_plugin.rs` | MainMenu, Selection hub, Map/Build pickers, GameOver overlay |
| `DesignPlugin` | `plugins/design_plugin.rs` | Design workshop: part editors, build assembly, part management |
| `UiPlugin` | `plugins/ui_plugin.rs` | In-game HUD (HP, velocity, phase text) |
| `StoragePlugin` | `plugins/storage_plugin.rs` | SQLite/SQLx init, TokioRuntime resource |

---

## FixedUpdate Pipeline (Battle phase only)

SystemSets in strict chain order:

```
1. PhysicsSet (chained):
   integrate_physics -> integrate_projectiles -> spin_drain ->
   tick_control_state -> tick_status_effects -> tick_melee_trackers ->
   wall_reflection

2. CollisionDetectSet:
   detect_collisions

3. EventGenerateSet (chained):
   generate_collision_damage -> detect_melee_hits -> fire_ranged_weapons

4. HookProcessSet:
   process_hooks (v0 no-op)

5. EventApplySet (chained):
   apply_damage_events -> apply_control_events -> resolve_top_collisions ->
   spawn_obstacles -> spawn_projectiles

6. CleanupSet (chained):
   despawn_projectiles_outside_arena -> cleanup_ttl -> handle_despawn_events -> play_sound_effects
```

---

## Key Resources

| Resource | Description |
|----------|-------------|
| `Tuning` | All tunable params, loaded from `tuning.ron`, F5 hot-reload |
| `PartRegistry` | Data-driven part presets (tops, weapons, shafts, builds, etc.) + `BuildRef` entries |
| `GameSelection` | Current mode, map, P1/P2 build IDs |
| `PickingFor` | Which player (1 or 2) is in the picker screen |
| `ProjectileAssets` | Projectile mesh/material + per-weapon sprite handles |
| `GameAssets` | All sprite handles + SFX handles, loaded at startup |
| `DesignState` | Current state of the design workshop (editing part ID, build assembly slots, etc.) |
| `SqliteRepo` | SQLite-backed repository for parts and builds |
| `TokioRuntime` | Tokio runtime for async-to-sync bridge |

---

## Build System

Players select **builds** (not individual tops + weapons). A build is a complete loadout:

```
Build = Top + Weapon + Shaft + Chassis + Screw
```

### BuildRef (in-memory)
`PartRegistry.builds` stores `BuildRef` entries with part IDs. Resolved to full `Build` structs at arena setup time via `resolve_build()`.

### Default Builds
| Build ID | Name | Top | Weapon |
|----------|------|-----|--------|
| `default_blade` | Standard Top + Blade | default_top | basic_blade (Melee) |
| `default_blaster` | Standard Top + Blaster | default_top | basic_blaster (Ranged) |

### Custom Builds
Created via Design Workshop → Assemble Build. Saved to SQLite `builds` table. Loaded into `PartRegistry.builds` at startup via `merge_custom_builds()`.

---

## Entity Lifecycle

- **InGame marker**: All game-session entities tagged with `InGame` component
- **Cleanup**: `cleanup_game` on `OnEnter(MainMenu)` despawns all `InGame` entities
- **Projectiles**: Despawned on hit, lifetime expiry, or leaving arena boundary
- **Obstacles**: Despawned when `ExpiresAt` time is reached

---

## Message System (Bevy B0002 workaround)

- `CollisionMessage`: Top-Top collision data (separate type to avoid Res/ResMut conflict)
- `GameEvent`: DealDamage, ApplyControl, ApplyStatus, SpawnProjectile (includes `weapon_id` for sprite lookup), SpawnObstacle, DespawnEntity

---

## Stats Architecture (3-Layer)

1. `BaseStats` — immutable base parameters per top
2. `ModifierSet` — from parts + passive traits + status
3. `EffectiveStats` — Base + modifiers applied; cached, recomputed on loadout change

---

## Asset System

### Convention-Based Loading
- Top ID `"default_top"` -> `assets/tops/default_top.png`
- Weapon ID `"basic_blade"` -> `assets/weapons/basic_blade.png`
- Ranged weapon `"basic_blaster"` -> `assets/projectiles/basic_blaster_projectile.png`
- Override via optional `sprite_path` / `projectile_sprite_path` fields in `BaseStats` / `WeaponWheelSpec`

### Fallback Strategy
- **Missing image** -> procedural mesh with fallback color (game renders identically to pre-sprite era)
- **Missing audio** -> silence (Bevy handles missing audio gracefully)
- No code changes needed to add new tops — just drop `{id}.png` in `assets/tops/`

### Rendering
- Game entities: `Sprite { image, custom_size }` (world-unit sized), else `Mesh2d` + `MeshMaterial2d`
- UI previews: `ImageNode` in picker/editor cards, else colored `Node` with `BackgroundColor`

### Audio
- `SfxHandles` holds 6 sound effect handles: launch, collision_top, collision_wall, melee_hit, ranged_fire, projectile_hit
- `play_sound_effects` system in CleanupSet reads `GameEvent` + `CollisionMessage`, spawns one-shot `AudioPlayer::<AudioSource>` with `PlaybackSettings::DESPAWN`
- Launch sound played in `launch_tops()` on battle entry

### Asset Directory Structure
```
assets/
  tops/           # {top_id}.png
  weapons/        # {weapon_id}.png
  projectiles/    # {weapon_id}_projectile.png
  shafts/         # {shaft_id}.png
  chassis/        # {chassis_id}.png
  screws/         # {screw_id}.png
  ui/             # edit.png, edit_hover.png, delete.png, delete_hover.png
  audio/sfx/      # launch.ogg, collision_top.ogg, collision_wall.ogg,
                  # melee_hit.ogg, ranged_fire.ogg, projectile_hit.ogg
```

### Image Specifications

All images must be **PNG format with RGBA** (transparent background recommended).

| Asset Type | Recommended Size | Orientation | Notes |
|------------|-----------------|-------------|-------|
| Top | 128x128 px | Facing right (+X) | Displayed as circle in-game; `custom_size` scales to world radius. UI preview: 80px in build picker, 64px in manage cards, 96px in editors |
| Weapon | 128x32 px (melee) / 64x32 px (ranged) | Facing right (+X) | Width = blade/barrel length, Height = thickness. Scaled to match weapon spec dimensions in-game |
| Projectile | 32x32 px | Any | Scaled via `Transform.scale` to match projectile radius in-game |
| Shaft / Chassis / Screw | 128x128 px | Any | Only shown in UI previews (32px in build assembly slots, 64px in manage cards, 96px in editors). Not rendered in-game |
| UI Icons | 32x32 px | N/A | Used for edit/delete buttons. Provide normal + hover variants (`{name}.png`, `{name}_hover.png`) |

**Key points:**
- Images are auto-loaded by convention: part ID determines file path
- Oversized images work but waste memory — keep under 512x512 for parts
- The `rfd` file dialog in the design workshop copies user-selected images to the correct `assets/` subdirectory

---

## Design Workshop

### Part CRUD Flow
1. User creates a part via editor (weapon/shaft/chassis/screw)
2. Part spec saved as JSON to SQLite `parts` table
3. Part registered in `PartRegistry` in-memory
4. Optional: assign image via file dialog (copies to `assets/{slot}/`)

### Build Assembly Flow
1. User picks parts for each slot (top, weapon, shaft, chassis, screw)
2. Build saved to SQLite `builds` table
3. `BuildRef` registered in `PartRegistry.builds` in-memory
4. Build immediately available in game's build picker

### Part Deletion
- **Referential integrity**: blocks deletion if part is used by any build (shows error banner)
- Removes from SQLite + in-memory registry
- Deletes associated image file from `assets/{slot}/`
- For weapons: also deletes projectile image from `assets/projectiles/`

---

## Key Design Decisions

- **Coordinate system**: All positions/sizes in world units. Camera `OrthographicProjection` with `scale = 1/pixels_per_unit` handles zoom.
- **Elastic collisions**: `wall_bounce_damping = 1.0` and `top_collisions_restitution = 1.0` by default.
- **Weapon visuals**: Spawned as child entities of tops. Parent rotation auto-rotates children.
- **Projectile visuals**: Sprite if weapon has projectile sprite, else unit circle mesh scaled via `Transform.scale`.
- **Data-driven parts**: `PartRegistry` holds all parts by ID. `setup_arena()` looks up `BuildRef` by build ID, then calls `resolve_build()` to assemble the full `Build`.
- **Build-based selection**: Players select complete builds (top + all parts), not individual tops + weapons separately.
- **Initial aim direction**: Each top starts aimed toward the opponent (P1: angle 0, P2: angle PI).

---

## Bevy 0.18 API Notes

- `ChildSpawnerCommands` (not `ChildBuilder`) for `with_children` closure parameter
- `despawn()` is recursive by default (`despawn_recursive()` was removed)
- `BorderRadius` is a field on `Node`, not a standalone Component
- Bundle tuple max size ~15 elements — nest tuples if needed
- Use `MessageWriter`/`MessageReader` for game events (not `Events`)
- `MessageReader<KeyboardInput>` for keyboard events in custom text input (not `EventReader`)
- Query conflict (B0001): use `Without<T>` to prove disjointness; `Changed<T>` does NOT help
