# Cyber Top - Game Design Spec

> Spinning tops battle in a frictionless circular arena.

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
- Projectile despawns on hit or when leaving arena boundary.

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
