# Cyber Top - Development Guide

> Rust + Bevy desktop game: spinning tops battle in a frictionless circular arena.
> Single-player offline. SQLite + SQLx for data persistence.

---

## Tech Stack

- **Language**: Rust
- **Game Framework**: Bevy (ECS / Schedules / Events / Assets / Input / Render)
- **Database**: SQLite + SQLx (async, offline compile with `sqlx-data.json`)
- **Tuning Format**: RON (`data_dir()/tuning.ron`), key-reload at runtime

---

## Architecture Principles

1. **Event-driven resolution**: collisions/attacks produce events, never directly modify HP.
2. **Data-driven specs**: parts/traits/obstacles/weapons defined via spec + modifiers + hooks.
3. **Compile-time safety**: use `struct`/`enum` + `match` to force branch coverage on new features.
4. **No DB IO in combat tick**: only read in-memory `EffectiveStats` and runtime state.
5. **All tunable params in one tuning file**: spin drain, reflection damping, control multiplier rules, etc.

---

## Core Gameplay

### Top Parameters
- **Spin (RPM) = HP**: spin reaches 0 → out.
- **Size (Radius)**: affects natural spin drain and damage-taken multiplier.
- **Move Speed**
- **Control Reduction** (`r`): multiplicative stacking `R = Π(1 + r_i) - 1`, effective multiplier `m = max(0, 1 - R)`.

### Combat Rhythm (Design Intent)
- Natural spin drain: **small**.
- Obstacle/wall collision spin cost: **small**.
- Differentiation comes from weapons/parts/traits.

---

## Physics Model (v0: Frictionless)

- **No friction**: velocity only changes via input, collision, or external forces.
- **Circular arena** with reflective walls.
- `wall_bounce_damping` ≈ 1.0 (configurable in tuning).
- Speed clamped to `max_speed` after every update and wall reflection.

---

## Bevy Schedule (Mandatory Order)

### Startup
- Load/create tuning, init SQLite, run migrations, load/create default build, build EffectiveStats cache, load skin asset map.

### Update
1. Read input (player/AI)
2. Update UI/Debug (loadout, tuning reload, data display)
3. Write `Intent` components/resources (intent only, no physics mutation)

### FixedUpdate (sole place that mutates combat state)
SystemSets in strict order:
1. `InputIntentSet` — consume Intent → apply acceleration/turn
2. `PhysicsSet` — integrate velocity/position/angle
3. `CollisionDetectSet` — Top–Top, Top–Wall, Top–Obstacle, Projectile–*, Melee hitbox–Top
4. `EventGenerateSet` — produce `GameEvent`s from collisions
5. `HookProcessSet` — parts hooks → status hooks → floor hooks → global rules
6. `EventApplySet` — apply events to state (spin/control/status)
7. `CleanupSet` — TTL expiry, despawn

---

## Entity Types
- `Top` — the spinning top (player/AI controlled)
- `Projectile` — ranged weapon output
- `Obstacle` — timed environmental objects
- (Future) `FloorZone` — area effects

### Top Runtime State (Minimum)
- Position / velocity vector
- Rotation angle (for weapon/projectile direction)
- `spin_hp_current`
- Control state (stun/slow remaining)
- Status effect instances (buff/debuff)
- Build ID (or inline 4 part IDs)
- `EffectiveStats` (pre-computed cache, read-only during tick)

---

## Stats Architecture (3-Layer, Cacheable)

1. `BaseStats` — immutable base parameters
2. `ModifierSet` — from parts + passive traits + status + floor
3. `EffectiveStats` — Base + modifiers applied; cached, recomputed on loadout change or battle entry

### Modifier Stacking
- `StatAdd` (default 0)
- `StatMul` (default 1)
- `StatClamp` (optional)

---

## Equipment System (4 Parts)

| Slot | Role |
|------|------|
| **Weapon Wheel** | Weapon type & attack spec |
| **Shaft** | Stability, spin efficiency |
| **Chassis** | Move speed, acceleration |
| **Trait Screw** | Passive buffs + event hooks |

### Shaft Stats (v0)
- `stability`: reduces collision displacement
- `spin_efficiency`: reduces idle spin drain

### Trait Screw Hooks
- `on_hit`: attach debuff
- `on_tick`: spawn obstacle
- `on_wall_collision`: extra wall damage
- `on_fire_projectile`: fire rate/spread adjustment

---

## Damage Model

### Unified Entry Point
All damage as events: `DealDamage { src, dst, amount, kind, tags }`
- `kind`: `Collision | Melee | Projectile | Wall | Obstacle`

### Resolution Order (per DealDamage)
1. `amount *= src_damage_out_mult` (source output multiplier)
2. `amount *= dst_damage_in_mult` (target intake multiplier)
3. `amount = clamp(amount, 0, +∞)`
4. `dst.spin_hp = max(0, spin_hp - amount)`

### Melee Damage
`DealDamage { kind: Melee, amount: base_damage * melee_damage_scale }`
- `melee_damage_scale = weapon_dmg_out_mult * hit_speed_scale`
- `hit_speed_scale = 1 + k * rel_speed` (k from tuning)

### Projectile Damage
`DealDamage { kind: Projectile, amount: projectile_damage_base }`
- Projectile despawns on hit (default)

### Collision Damage
`collision_damage = tuning.collision_damage_k * rel_speed`
- Wall: `wall_damage_k * rel_speed` (default ≈ 0)
- Obstacle: `ObstacleSpec.damage_on_hit`

### Size → Damage Taken
`dst_damage_in_mult = 1 + size_damage_k * (dst.radius - size_radius_ref)`

---

## Weapon System

### Types: `Melee | Ranged | (Future) Hybrid`

### Aim Modes
- `FollowSpin`: direction = top rotation angle
- `SeekNearestTarget`: direction = toward nearest target (future)

### Ranged Spec
- Rate of fire, burst/spread, spread angle, knockback distance
- Projectile radius, control duration, range/lifetime, AimMode

### Melee Spec
- Base damage, hit cooldown, max hits per rotation
- Hitbox (radius/angle), hit control (stun/slow)

### Control Effects
- `Stun { duration }`, `Slow { duration, ratio }`, `Knockback { distance }`
- **Knockback is a control effect** → subject to control reduction multiplier `m`
- `effective_duration = base_duration * m`
- `effective_distance = distance * m`

---

## Obstacle System (TTL-Based)

### Instance Fields
- `id`, `owner` (optional), `spawn_time`, `expires_at`
- `shape`: Circle (expandable to rect/polygon)
- `collision_behavior`: `Solid | DamageOnHit(amount) | ApplyControlOnHit(control)`

### Cleanup: each tick, if `now >= expires_at` → `DespawnEntity`

---

## Event System

### GameEvent Enum (Minimum)
- `Collision { a, b, impulse, normal }`
- `DealDamage { src, dst, amount, kind }`
- `ApplyControl { src, dst, control }`
- `ApplyStatus { src, dst, status }`
- `SpawnProjectile { src, spec }`
- `SpawnObstacle { src, spec, ttl }`
- `DespawnEntity { id }`

### Hook Pipeline Order
1. Equipment part hooks (4 parts)
2. Status effect hooks (buff/debuff)
3. Floor/arena hooks (future)
4. Global rules (tuning, cap, clamp)

---

## Newtypes (Unit Types)
- `SpinHp(f32)` — `add_clamped`, `sub_clamped`, `is_finite` check
- `Radius(f32)`
- `MetersPerSec(f32)`
- `Seconds(f32)` — `dec(dt)` → `max(0)`
- `Multiplier(f32)`
- `AngleRad(f32)` — `rem_euclid(TAU)` each tick
- `Tick(u64)` — `next()` via `checked_add(1)`

---

## Numerical Safety

### f32 (continuous quantities)
- Clamp after every update; `debug_assert!(is_finite())`
- `SpinHp`: `[0, spin_hp_max]`
- `Multiplier`: `[0.0, 10.0]`
- `MoveSpeed`: `[0, max_speed]`

### u64 (discrete counters)
- `tick_index`, `event_seq`: `checked_add` (panic on overflow = bug)
- UI/stats counters: `saturating_add`

### Mandatory Clamp List
- `wall_bounce_damping`: `[0.0, 1.0]`
- `damage_taken_mult`: `[0.0, 10.0]`
- `fire_rate_mult`: `[0.1, 10.0]`
- `stability`: `[0, stability_max]`
- Spin drains: non-negative, upper-bounded

---

## DB Schema (SQLite, JSON blob + balance_version)

### Tables
- `tops`: id, base_stats_json, skin_id, balance_version
- `parts`: id, slot, kind, spec_json, balance_version
- `builds`: id, top_id, weapon_id, shaft_id, chassis_id, screw_id, note
- `effective_cache`: build_id, effective_stats_json, computed_at, balance_version, hash

### Cache Strategy
- Compute on loadout change / battle entry → write `effective_cache`
- Combat tick: read-only from memory cache
- Invalidate via `hash` / `balance_version`

---

## Project Structure

```
src/
  main.rs
  plugins/
    mod.rs
    game_plugin.rs      # FixedUpdate/Update/Startup system registration & SystemSet ordering
    ui_plugin.rs        # Loadout/Debug UI
    storage_plugin.rs   # SQLite/SQLx init, repo resource
  game/
    mod.rs
    components.rs       # Top/Projectile/Obstacle components
    intent.rs           # Input intent components/resources
    tick.rs             # FixedUpdate system collection
    physics.rs          # Movement, reflection, collision primitives
    collision.rs        # Broad/narrow phase
    combat.rs           # Event generation (no direct state mutation)
    events.rs           # Bevy Events (GameEvent etc.)
    hooks.rs            # Hook trait + dispatcher
    stats/
      types.rs
      base.rs
      modifier.rs
      effective.rs
    parts/
      mod.rs
      weapon_wheel.rs
      shaft.rs
      chassis.rs
      trait_screw.rs
    status/
      mod.rs
      effect.rs
    arena/
      mod.rs
      circle.rs
      obstacle.rs
      floor.rs           # future
  storage/
    mod.rs
    repo.rs             # BuildRepository trait (Resource)
    sqlite_repo.rs      # SQLx implementation (Resource)
  config/
    tuning.rs           # Tuning load/reload (Resource)
  assets_map.rs         # skin_id → Handle<Image> / VisualSpec mapping (Resource)
migrations/
assets/
  skins/
```

---

## Tuning Defaults (tuning.ron)

```ron
(
  dt: 0.0166667,
  arena_radius: 12.0,
  wall_bounce_damping: 0.99,
  spin_drain_idle_per_sec: 0.2,
  spin_drain_on_wall_hit: 0.5,
  spin_drain_on_top_hit: 1.0,
  collision_damage_k: 0.5,
  wall_damage_k: 0.0,
  size_damage_k: 0.0,
  size_radius_ref: 1.0,
  max_speed: 8.0,
  input_accel: 25.0,
)
```

---

## MVP Milestones (Delivery Order)

1. World + entity storage (Top/Obstacle/Projectile)
2. Basic tick pipeline
3. Frictionless movement + circular wall reflection
4. Collision detection (Top–Wall, Top–Top, Top–Obstacle)
5. Event system (Collision/DealDamage/ApplyControl/SpawnProjectile/Despawn)
6. Stats: Base → Effective (cached) + tuning-controlled spin drain
7. 4-part data model + 1–2 example parts per slot
8. WeaponWheel: 1 simple Melee + 1 simple Ranged (FollowSpin)
9. Obstacle TTL: spawn + expiry
10. SQLite + SQLx: migrations, parts/tops/builds/effective_cache
11. Placeholder rendering (shape + color by skin_id)
