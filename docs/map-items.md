# Map Items Reference

> Each item placed in the Map Design editor appears in battle as a physical entity.
> All items are tagged `InGame` and cleaned up automatically when the battle ends.

---

## Grid System

- **Cell size**: 0.5 world units per grid cell
- **World position**: `(grid_x × 0.5, grid_y × 0.5)`, origin = arena center
- **Valid placement**: cell center must be at least 0.25 units inside the arena boundary
- **Arena radius**: configurable per map, default 12.0 world units

---

## Item Types

### Obstacle (Gray)

**Purpose**: Static wall that blocks and damages tops on contact.

| Property | Value |
|----------|-------|
| Visual | 0.5 × 0.5 gray square |
| Collision radius | 0.25 world units (exact cell boundary) |
| Bounce | Elastic reflection: `v' = v − 2(v·n̂)n̂` |
| Damage on hit | `tuning.obstacle_damage` (default 2.0 spin HP) |
| Persistence | Permanent for the duration of the battle |

**Behavior**: When a top overlaps the obstacle:
1. `static_obstacle_bounce` (PhysicsSet) pushes the top out and reflects its velocity elastically.
2. `detect_collisions` (CollisionDetectSet) emits a `DealDamage` event with `DamageKind::Obstacle`.

---

### Gravity Device (Purple)

**Purpose**: Continuously steers tops toward itself while in range.

| Property | Value |
|----------|-------|
| Visual | Circle with radius 2.0 (semi-transparent purple) |
| Detection radius | 2.0 world units from center |
| Steer strength | 3.0 (fraction of direction blended per second) |
| Speed preserved | Yes — only direction is altered, not magnitude |
| Persistence | Permanent |

**Behavior**: Each FixedUpdate tick, for every top within `device.radius + top_radius`:
```
blend = clamp(steer_strength × dt, 0.0, 1.0)
new_dir = normalize(current_dir × (1 - blend) + toward_device × blend)
vel = new_dir × speed
```
The effect is a smooth continuous pull; tops orbit the device if their speed is sufficient.

---

### Speed Boost Zone (Green)

**Purpose**: Temporarily increases a top's effective movement speed.

| Property | Value |
|----------|-------|
| Visual | Circle with radius 1.0 (semi-transparent green) |
| Detection radius | 1.0 world units (threshold = top_radius + 1.0 ≈ 2.3 units) |
| Speed multiplier | 1.5× (50% faster) |
| Duration | 3.0 seconds after last touching the zone |
| Component affected | `SpeedBoostEffect.multiplier` on the top |

**Behavior**: `speed_boost_system` (first in PhysicsSet) checks overlap each tick. While overlapping, sets `SpeedBoostEffect { multiplier: 1.5, expires_at: now + 3.0 }` directly on the top. `integrate_physics` then uses `eff_vel = vel × multiplier` for position integration. The raw `Velocity` component is unchanged; only the position delta (and visual spin) are scaled.

After 3 seconds without re-entering a zone, `speed_boost_tick` resets `multiplier` to 1.0.

**HUD**: `spd:` shows effective speed (`vel.length() × multiplier`). Jumps from ~10.2 to ~15.3 while boosted.

---

### Damage Boost Zone (Red)

**Purpose**: Increases the outgoing weapon damage of any top touching the zone.

| Property | Value |
|----------|-------|
| Visual | Circle with radius 1.0 (semi-transparent red) |
| Detection radius | 1.0 world units |
| Damage multiplier | 1.5× outgoing damage |
| Duration | Active only while overlapping (no persistence) |
| Component affected | `DamageBoostActive.multiplier` on the top |

**Behavior**: `damage_boost_system` (PhysicsSet) checks overlap each tick. While overlapping, sets `DamageBoostActive { multiplier: 1.5 }`. When not overlapping, resets to 1.0. Applied in `apply_damage_events` (EventApplySet):
```
final_damage = base × damage_out_mult × dmg_boost.multiplier × damage_in_mult
```
Affects all damage types the top deals: collision, melee, ranged projectiles.

**HUD**: `wpn:` shows effective weapon damage (`base × out_mult × boost_mult`). Jumps ~50% while in zone.

---

## System Execution Order

Zone systems run at the start of `PhysicsSet` (before `integrate_physics`), so multipliers are applied within the same FixedUpdate tick as the movement they affect:

```
speed_boost_system       ← sets SpeedBoostEffect.multiplier
speed_boost_tick         ← resets expired effects
damage_boost_system      ← sets DamageBoostActive.multiplier
gravity_device_system    ← steers velocity toward device
integrate_physics        ← applies eff_vel = vel × speed_mult
...
```

`DamageBoostActive` is applied later in `EventApplySet → apply_damage_events`.

---

## Design Notes

- All zone items spawn with `CollisionRadius(1.0)` — the same value used for detection overlap.
- Obstacles use `CollisionRadius(0.25)` matching the 0.5-unit cell size exactly.
- Both `SpeedBoostEffect` and `DamageBoostActive` are **always-present** components on tops (initialized with `multiplier: 1.0` at spawn). Zone systems mutate them directly — no `Commands.insert/remove` deferred overhead.
- Multiple overlapping zones of the same type: best (highest) multiplier wins.
