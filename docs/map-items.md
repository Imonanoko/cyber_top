# Map Items Reference

> Each item placed in the Map Design editor appears in battle as a physical entity.
> All items are tagged `InGame` and cleaned up automatically when the battle ends.
> Sprites live in `assets/obstacles/` (64×64 RGBA PNG, regenerated via `python3 gen_assets.py`).

---

## Grid System

- **Cell size**: 0.5 world units per grid cell
- **World position**: `(grid_x × 0.5, grid_y × 0.5)`, origin = arena center
- **Valid placement**: cell center must be at least 0.25 units inside the arena boundary
- **Arena radius**: configurable per map, default 12.0 world units

---

## Item Types

### Obstacle (Gray X icon)

**Purpose**: Static wall that blocks and damages tops on contact.

| Property | Value |
|----------|-------|
| Sprite | `assets/obstacles/obstacle.png` — gray X on dark background |
| Visual size | 0.5 × 0.5 world units (one grid cell) |
| Collision radius | 0.25 wu (half cell — exact cell boundary) |
| Editor stamp | 1 × 1 cell |
| Bounce | Elastic reflection: `v' = v − 2(v·n̂)n̂` |
| Damage on hit | `tuning.obstacle_damage` (default 2.0 spin HP) |
| Persistence | Permanent for the duration of the battle |

**Behavior**: When a top overlaps the obstacle:
1. `static_obstacle_bounce` (PhysicsSet) pushes the top out and reflects its velocity elastically.
2. `detect_collisions` (CollisionDetectSet) emits a `DealDamage` event with `DamageKind::Obstacle`.

---

### Gravity Device (Purple rings icon)

**Purpose**: Continuously steers tops toward itself while in range.

| Property | Value |
|----------|-------|
| Sprite | `assets/obstacles/gravity_device.png` — purple concentric rings |
| Visual size | 6.0 × 6.0 wu (diameter of effect radius) |
| Detection radius | 3.0 wu from device center |
| Steer strength | 3.0 (direction blended per second) |
| Speed preserved | Yes — only direction is altered, not magnitude |
| Editor stamp | 1 × 1 cell |
| Persistence | Permanent |

**Behavior**: Each FixedUpdate tick, for every top within `device.radius + top_radius`:
```
blend = clamp(steer_strength × dt, 0.0, 1.0)
new_dir = normalize(current_dir × (1 - blend) + toward_device × blend)
vel = new_dir × speed
```
The effect is a smooth continuous pull; tops orbit the device if their speed is sufficient.

---

### Speed Boost Zone (Green lightning bolt icon)

**Purpose**: Temporarily increases a top's effective movement speed.

| Property | Value |
|----------|-------|
| Sprite | `assets/obstacles/speed_boost.png` — yellow-green lightning bolt |
| Visual size | 0.5 × 0.5 wu per tile |
| Collision radius | 0.25 wu per tile (half cell) |
| Detection threshold | `top_radius + 0.25 ≈ 1.55 wu` from tile center |
| Editor stamp | **2 × 2 cells** (4 tiles placed per click) |
| Speed multiplier | 1.5× |
| Duration | 3.0 seconds after last contact with any tile |
| Component affected | `SpeedBoostEffect.multiplier` on the top |

**Behavior**: `speed_boost_system` (first in PhysicsSet) checks overlap each tick. While overlapping, sets `SpeedBoostEffect { multiplier: 1.5, expires_at: now + 3.0 }` directly on the top. `integrate_physics` then uses `eff_vel = vel × multiplier` for position integration. The raw `Velocity` component is unchanged; only the position delta (and visual spin rate) are scaled.

After 3 seconds without re-entering a tile, `speed_boost_tick` resets `multiplier` to 1.0.

**Coverage**: To create a larger zone, place more 2×2 stamps side-by-side. Each tile independently triggers the boost; best (highest) multiplier across all overlapping tiles wins.

**HUD**: `spd:` shows effective speed (`vel.length() × multiplier`) — jumps ~50% while boosted.

---

### Damage Boost Zone (Red sword icon)

**Purpose**: Increases outgoing weapon damage for any top touching the zone.

| Property | Value |
|----------|-------|
| Sprite | `assets/obstacles/damage_boost.png` — white sword on dark red |
| Visual size | 0.5 × 0.5 wu per tile |
| Collision radius | 0.25 wu per tile (half cell) |
| Detection threshold | `top_radius + 0.25 ≈ 1.55 wu` from tile center |
| Editor stamp | **2 × 2 cells** (4 tiles placed per click) |
| Damage multiplier | 1.5× outgoing damage |
| Duration | Active only while overlapping any tile (no persistence after leaving) |
| Component affected | `DamageBoostActive.multiplier` on the top |

**Behavior**: `damage_boost_system` (PhysicsSet) checks overlap each tick. While overlapping, sets `DamageBoostActive { multiplier: 1.5 }`. When not overlapping, resets to 1.0. Applied in `apply_damage_events` (EventApplySet):
```
final_damage = base_damage × damage_out_mult × dmg_boost.multiplier × damage_in_mult
```
Affects all damage types the top deals: collision, melee, and ranged projectiles.

**HUD**: `wpn:` shows effective weapon damage (`base × out_mult × boost_mult`) — jumps ~50% while in zone.

---

## System Execution Order

Zone systems run at the start of `PhysicsSet` (before `integrate_physics`), so multipliers are applied within the same FixedUpdate tick as the movement they affect:

```
speed_boost_system       ← sets SpeedBoostEffect.multiplier (logs "SpeedBoost ACTIVATED" on entry)
speed_boost_tick         ← resets expired effects to multiplier 1.0
damage_boost_system      ← sets DamageBoostActive.multiplier (logs "DamageBoost ACTIVATED" on entry)
gravity_device_system    ← blends velocity direction toward device
integrate_physics        ← applies eff_vel = vel × speed_mult (logs speed values once/sec when active)
...
```

`DamageBoostActive` is applied later in `EventApplySet → apply_damage_events` (logs boosted vs base damage per hit).

---

## Design Notes

- All map items use `CollisionRadius(cell_radius)` = `GRID_CELL_SIZE × 0.5` = **0.25 wu**.
- Zone coverage scales by tile count: stamp more 2×2 blocks to make a larger area.
- Both `SpeedBoostEffect` and `DamageBoostActive` are **always-present** components on tops (initialized `multiplier: 1.0` at spawn). Zone systems mutate them directly — no `Commands.insert/remove` deferred overhead.
- Multiple overlapping tiles of the same type: best (highest) multiplier wins.
- Sprite regeneration: edit `gen_assets.py` and run `python3 gen_assets.py` — no pip install needed.
