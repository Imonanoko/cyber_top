# Cyber Top - Project Architecture

## Overview

Cyber Top is a 2D spinning-tops battle game built with **Rust + Bevy 0.18**.
Two tops are launched into a circular arena; physics (collisions, wall reflections) drives all movement after launch. The last top with spin HP > 0 wins.

---

## Directory Structure

```
src/
  main.rs                     # App entry point, window + plugin setup
  assets_map.rs               # Skin color lookup table (placeholder for sprites)
  config/
    mod.rs
    tuning.rs                 # All tunable constants (loaded from tuning.ron)
  game/
    mod.rs
    components.rs             # ECS marker components, state, runtime data
    events.rs                 # Message types (GameEvent, CollisionMessage)
    collision.rs              # Collision detection systems
    combat.rs                 # Damage generation, collision resolution, ranged/melee
    physics.rs                # Integration, spin drain, status ticking
    hooks.rs                  # Hook pipeline (v0 no-op, v0.2+)
    intent.rs                 # (deprecated) — movement is now launch-based
    tick.rs                   # Reserved for tick-level utilities
    arena/
      mod.rs
      circle.rs               # Wall reflection + wall damage
      obstacle.rs             # Obstacle/projectile spawning, TTL cleanup, despawn
      floor.rs                # (placeholder) Floor zone effects
    parts/
      mod.rs                  # Build struct (weapon + shaft + chassis + screw)
      registry.rs             # PartRegistry resource (data-driven part presets)
      weapon_wheel.rs         # MeleeSpec, RangedSpec, WeaponWheelSpec
      shaft.rs                # ShaftSpec (stability, spin efficiency)
      chassis.rs              # ChassisSpec (move speed modifiers)
      trait_screw.rs          # TraitScrewSpec (passive stats, event hooks)
    stats/
      mod.rs
      types.rs                # Newtypes: SpinHp, Radius, MetersPerSec, AngleRad, etc.
      base.rs                 # BaseStats (immutable top base parameters)
      effective.rs            # EffectiveStats (pre-computed from base + modifiers)
      modifier.rs             # StatModifier, ModifierSet, compute_effective()
    status/
      mod.rs
      effect.rs               # StatusEffectDef (placeholder)
  plugins/
    mod.rs
    game_plugin.rs            # Game systems, scheduling, setup, aiming, launch, game over
    ui_plugin.rs              # HUD: HP display, phase text, game over screen
    storage_plugin.rs         # SQLite init for build persistence
  storage/
    mod.rs
    repo.rs                   # BuildRepository trait
    sqlite_repo.rs            # SQLite implementation
```

---

## File Details

### `src/main.rs`
- Creates Bevy `App` with `DefaultPlugins`, window (800x800)
- Inserts `Tuning` and `AssetsMap` resources
- Adds `GamePlugin`, `UiPlugin`, `StoragePlugin`

### `src/assets_map.rs`
- `AssetsMap` resource: maps skin IDs to `Color` values
- Placeholder for future sprite/texture lookup

### `src/config/tuning.rs`
- `Tuning` struct: **all** tunable parameters in one place
- Loaded from `tuning.ron` (RON format) in user data dir
- F5 hot-reload at runtime
- Key parameters:
  - `pixels_per_unit` — camera zoom (1 world unit = N pixels)
  - `arena_radius` — circular arena radius (world units)
  - `wall_bounce_damping` — velocity multiplier on wall hit (1.0 = elastic)
  - `top_collisions_restitution` — restitution coefficient (1.0 = elastic)
  - `collision_damage_k`, `wall_damage_k` — damage scaling
  - `obstacle_damage` — default obstacle contact damage
  - `aim_speed` — aim rotation speed (rad/s)
  - `spin_visual_k` — visual spin rate multiplier

### `src/game/components.rs`
- **Markers**: `Top`, `ProjectileMarker`, `ObstacleMarker`, `PlayerControlled`, `AiControlled`, `AimArrow`, `WeaponVisual`
- **GamePhase**: `Aiming` -> `Battle` -> `GameOver` (Bevy States)
- **LaunchAim**: angle + confirmed flag for aiming phase
- **Runtime**: `Velocity`, `RotationAngle`, `SpinHpCurrent`, `TopEffectiveStats`, `TopBuild`
- **Control/Status**: `ControlState` (stun/slow), `StatusEffects`
- **Projectile data**: `ProjectileDamage`, `ProjectileOwner`, `Lifetime`, `CollisionRadius`
- **Obstacle data**: `ObstacleOwner`, `ObstacleBehavior`, `ExpiresAt`
- **MeleeHitTracker**: per-target hit cooldowns
- **ProjectileAssets**: pre-built mesh/material for projectile rendering

### `src/game/events.rs`
- `CollisionMessage` — Top-Top collision data (separate message type to avoid Bevy B0002)
- `GameEvent` — DealDamage, ApplyControl, ApplyStatus, SpawnProjectile, SpawnObstacle, DespawnEntity
- `StatusEffectData`, `StatusEffectKind` — status effect data

### `src/game/collision.rs`
- `detect_collisions()` — runs in **CollisionDetectSet**
  - Top-Top: writes `CollisionMessage`
  - Top-Obstacle: writes `GameEvent::DealDamage`
  - Projectile-Top: writes `GameEvent::DealDamage` + `DespawnEntity`
  - Wall collision is handled by `circle::wall_reflection` (not here)

### `src/game/combat.rs`
- `generate_collision_damage()` — reads `CollisionMessage`, writes `GameEvent::DealDamage` (EventGenerateSet)
- `detect_melee_hits()` — checks melee weapon hitbox vs targets, writes damage/control events
- `fire_ranged_weapons()` — auto-fires on cooldown, writes `SpawnProjectile` events
- `apply_damage_events()` — reads `GameEvent::DealDamage`, applies to `SpinHpCurrent` (EventApplySet)
- `apply_control_events()` — reads `ApplyControl`, applies stun/slow
- `resolve_top_collisions()` — reads `CollisionMessage`, applies elastic velocity exchange + overlap separation
- `RangedFireTimer` — per-top cooldown component

### `src/game/physics.rs`
- `integrate_physics()` — velocity -> position, visual spin (`velocity × spin_visual_k × weapon.spin_rate_multiplier()`), syncs `RotationAngle` -> `Transform.rotation`
- `integrate_projectiles()` — moves projectiles, ticks lifetime
- `spin_drain()` — natural spin HP decay per second
- `tick_control_state()` — decrements stun/slow timers
- `tick_status_effects()` — decrements status effect timers
- `tick_melee_trackers()` — decrements melee hit cooldowns

### `src/game/arena/circle.rs`
- `wall_reflection()` — **authoritative** wall handler
  - Pushes top back inside arena boundary
  - Reflects velocity (elastic with `wall_bounce_damping`)
  - Generates wall damage event if `wall_damage_k > 0`

### `src/game/arena/obstacle.rs`
- `spawn_obstacles()` — reads `SpawnObstacle` events, creates obstacle entities
- `spawn_projectiles()` — reads `SpawnProjectile` events, creates **visible** projectile entities (with mesh)
- `cleanup_ttl()` — despawns expired obstacles and projectiles
- `handle_despawn_events()` — despawns entities from `DespawnEntity` events

### `src/game/parts/`
- `Build` — complete top configuration (top base stats + weapon + shaft + chassis + screw), holds `BaseStats` + full specs inline
- `registry.rs` — `PartRegistry` resource: stores tops and parts by ID in `HashMap<String, _>`. Provides `resolve_build()` to assemble a `Build` from IDs.
  - Preset tops: `"default_top"` (HP=100, radius=0.5, speed=5.0), `"small_top"` (HP=80, radius=0.35, speed=6.0)
  - Preset weapons: `"basic_blade"` (melee), `"basic_blaster"` (ranged)
  - Preset parts: `"standard_shaft"`, `"standard_chassis"`, `"standard_screw"`
- `weapon_wheel.rs` — weapon definition with optional `MeleeSpec` and `RangedSpec`. Each spec carries its own per-weapon parameters:
  - `MeleeSpec`: `blade_len`, `blade_thick` (visual size), `hitbox_radius`, `hitbox_angle` (collision), `spin_rate_multiplier`
  - `RangedSpec`: `barrel_len`, `barrel_thick` (visual size), `projectile_radius` (bullet size), `spin_rate_multiplier`
  - `WeaponWheelSpec::spin_rate_multiplier()` — returns the active spec's value (hybrid: max of both)
- `ShaftSpec` — stability + spin efficiency
- `ChassisSpec` — move speed + acceleration + collision radius modifiers (`move_speed_add/mul`, `accel_add/mul`, `radius_add/mul`)
- `TraitScrewSpec` — passive stat bonuses + event hooks

### `src/game/stats/`
- `types.rs` — Newtypes with safe arithmetic (`SpinHp`, `Radius`, `MetersPerSec`, `Seconds`, `Multiplier`, `AngleRad`)
- `base.rs` — `BaseStats` per-top base parameters (id, name, spin_hp_max, radius, move_speed, accel, control_reduction). Stored in `PartRegistry.tops` and `Build.top`.
- `effective.rs` — `EffectiveStats` pre-computed stats including `accel` (read during combat)
- `modifier.rs` — `StatModifier` (add/mul/clamp), `ModifierSet` (includes `accel`), `compute_effective()` pipeline

### `src/plugins/game_plugin.rs`
- **FixedGameSet** ordering: Physics -> CollisionDetect -> EventGenerate -> HookProcess -> EventApply -> Cleanup
- All FixedUpdate sets gated to `GamePhase::Battle`
- `setup_game()` — camera, arena, `PartRegistry`, player top (ranged via registry), AI top (melee via registry), weapon visuals, projectile assets
- `spawn_weapon_visual_mesh()` — creates weapon visual mesh + transform based on `WeaponKind`
- Aiming phase: `read_aim_input`, `ai_auto_aim`, `check_all_confirmed`, `update_aim_arrow`
- Battle entry: `launch_tops`, `despawn_aim_arrows`
- Game over: `check_game_over`

### `src/plugins/ui_plugin.rs`
- `HpText` — displays player + AI spin HP
- `PhaseText` — shows current phase / instructions
- `GameOverText` — shows winner on game over

### `src/plugins/storage_plugin.rs`
- Initializes SQLite database for build persistence
- Gracefully continues if DB init fails

---

## System Execution Order (FixedUpdate, Battle phase only)

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
   cleanup_ttl -> handle_despawn_events
```

---

## Game Flow

1. **Aiming** — Player rotates launch direction (Left/Right), presses Space to confirm. AI auto-confirms with random angle.
2. **Battle** — Tops launched at `move_speed` in chosen direction. All movement is physics-driven (elastic collisions, wall reflection). Ranged weapons auto-fire on cooldown. Melee hits on contact within hitbox angle.
3. **GameOver** — First top with spin HP <= 0 loses. Winner displayed.

---

## Key Design Decisions

- **Coordinate system**: All positions/sizes in world units. Camera `OrthographicProjection` with `scale = 1/pixels_per_unit` handles zoom.
- **Elastic collisions**: `wall_bounce_damping = 1.0` and `top_collisions_restitution = 1.0` by default — no speed loss from physics. Only specific features/traits should reduce speed.
- **Message split**: `CollisionMessage` is separate from `GameEvent` to avoid Bevy B0002 (Res/ResMut conflict on same resource within a system).
- **Weapon visuals**: Spawned as child entities of tops. Parent rotation (`RotationAngle` synced to `Transform.rotation`) automatically rotates children.
- **Projectile visuals**: Unit circle mesh scaled per-projectile via `Transform.scale`.
- **Data-driven parts**: `PartRegistry` resource holds all available parts by ID. `setup_game()` uses `resolve_build()` to assemble builds from part IDs. Aligns with DB schema (`parts` table with `spec_json`) for future DB loading.
