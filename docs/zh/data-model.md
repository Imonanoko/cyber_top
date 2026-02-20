# 資料模型 — 零件、配裝、數值、儲存

> 遊戲零件 / 配裝系統與持久化層的所有資料結構。

---

## 零件槽位

```rust
// src/game/stats/types.rs
pub enum PartSlot {
    WeaponWheel,   // 武器輪
    Shaft,         // 軸
    Chassis,       // 底盤
    TraitScrew,    // 特性螺絲
}
```

注意：輪盤**不是** PartSlot — 它是獨立概念（`BaseStats`）。

---

## 各槽位規格

### 輪盤 — `BaseStats`（`game/stats/base.rs`）

```rust
pub struct BaseStats {
    pub id: String,
    pub name: String,
    pub spin_hp_max: SpinHp,        // 預設 100.0
    pub radius: Radius,              // 預設 1.3
    pub move_speed: MetersPerSec,    // 預設 10.0
    pub accel: f32,                  // 預設 25.0
    pub control_reduction: f32,      // 預設 0.0
    pub sprite_path: Option<String>, // 覆蓋慣例路徑
}
```

### 武器 — `WeaponWheelSpec`（`game/parts/weapon_wheel.rs`）

```rust
pub struct WeaponWheelSpec {
    pub id: String,
    pub name: String,
    pub kind: WeaponKind,            // Melee（近戰）或 Ranged（遠程）
    pub melee: Option<MeleeSpec>,    // kind=Melee 時填充
    pub ranged: Option<RangedSpec>,  // kind=Ranged 時填充
    pub sprite_path: Option<String>,
    pub projectile_sprite_path: Option<String>,
}

pub struct MeleeSpec {
    pub base_damage: f32,            // 基礎傷害
    pub hit_cooldown: f32,           // 命中冷卻
    pub max_hits_per_rotation: u32,  // 每轉最大命中次數
    pub hitbox_radius: f32,          // 判定框半徑
    pub hitbox_angle: f32,           // 判定框角度（弧度，預設 PI/3）
    pub hit_control: Option<ControlEffect>,  // 命中控制效果
    pub spin_rate_multiplier: f32,   // 旋轉速率倍率
    pub blade_len: f32,              // 刀刃長度
    pub blade_thick: f32,            // 刀刃厚度
}

pub struct RangedSpec {
    pub projectile_damage: f32,      // 投射物傷害
    pub fire_rate: f32,              // 射速（發/秒）
    pub burst_count: u32,            // 連發數
    pub spread_angle: f32,           // 散射角度（弧度）
    pub knockback_distance: f32,     // 擊退距離
    pub projectile_radius: f32,      // 投射物半徑
    pub control_duration: Seconds,   // 控制持續時間
    pub lifetime: Seconds,           // 投射物存活時間
    pub projectile_speed: f32,       // 投射物速度
    pub aim_mode: AimMode,           // 瞄準模式
    pub spin_rate_multiplier: f32,
    pub barrel_len: f32,             // 砲管長度
    pub barrel_thick: f32,           // 砲管厚度
}
```

### 軸 — `ShaftSpec`（`game/parts/shaft.rs`）

```rust
pub struct ShaftSpec {
    pub id: String,
    pub name: String,
    pub stability: f32,          // 降低碰撞位移
    pub spin_efficiency: f32,    // 降低閒置旋轉消耗（倍率）
}
```

### 底盤 — `ChassisSpec`（`game/parts/chassis.rs`）

```rust
pub struct ChassisSpec {
    pub id: String,
    pub name: String,
    pub move_speed_add: f32,     // 速度加值（平坦）
    pub move_speed_mul: f32,     // 速度倍率（1.0 = 不變）
    pub accel_add: f32,          // 加速度加值
    pub accel_mul: f32,          // 加速度倍率
    pub radius_add: f32,         // 碰撞半徑加值
    pub radius_mul: f32,         // 碰撞半徑倍率
}
```

### 特性螺絲 — `TraitScrewSpec`（`game/parts/trait_screw.rs`）

```rust
pub struct TraitScrewSpec {
    pub id: String,
    pub name: String,
    pub passive: TraitPassive,
    pub hooks: Vec<TraitHookKind>,  // 未來：事件鉤子
}

pub struct TraitPassive {
    pub spin_hp_max_add: f32,       // 最大 HP 加值
    pub control_reduction: f32,     // 加入控制減免來源
    pub damage_out_mult: f32,       // 輸出傷害倍率
    pub damage_in_mult: f32,        // 承受傷害倍率
}
```

---

## 配裝系統

### BuildRef（輕量，記憶體中）— `game/parts/registry.rs`

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

### Build（已解析，完整規格）— `game/parts/mod.rs`

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

### 解析流程

```
BuildRef（僅 ID）
  → PartRegistry.resolve_build(build_id, build_name, top_id, weapon_id, shaft_id, chassis_id, screw_id)
  → Build（完整規格）
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
    pub maps: HashMap<String, MapSpec>,
}
```

### 生命週期

1. `PartRegistry::with_defaults()` — 填充硬編碼預設值
2. `merge_custom_parts(repo, rt)` — 從 SQLite `parts` 表載入（所有槽位 + 陀螺）
3. `merge_custom_builds(repo, rt)` — 從 SQLite `builds` 表載入
4. `merge_custom_maps(repo, rt)` — 從 SQLite `maps` 表載入
5. 執行時：編輯器同時儲存至 SQLite 並即時更新 HashMap

### 預設零件

| ID | 類型 |
|----|------|
| `default_top` | 輪盤 |
| `basic_blade` | 武器（近戰） |
| `basic_blaster` | 武器（遠程） |
| `standard_shaft` | 軸 |
| `standard_chassis` | 底盤 |
| `standard_screw` | 螺絲 |

### 預設配裝

| ID | 名稱 | 組成 |
|----|------|------|
| `default_blade` | Standard Blade Top | default_top + basic_blade + standard_* |
| `default_blaster` | Standard Blaster Top | default_top + basic_blaster + standard_* |

---

## 數值架構（3 層）

### 第 1 層：BaseStats
每個輪盤的不可變參數。儲存在 `PartRegistry.tops`。

### 第 2 層：ModifierSet（`game/stats/modifier.rs`）

```rust
pub struct StatModifier {
    pub add: f32,       // 加值（平坦）
    pub mul: f32,       // 乘值（預設 1.0）
    pub clamp_min: Option<f32>,
    pub clamp_max: Option<f32>,
}

pub struct ModifierSet {
    pub spin_hp_max: StatModifier,
    pub radius: StatModifier,
    pub move_speed: StatModifier,
    pub accel: StatModifier,
    pub control_reduction_sources: Vec<f32>,  // 乘法疊加
    pub stability: StatModifier,
    pub spin_efficiency: StatModifier,
    pub damage_out_mult: Multiplier,
    pub damage_in_mult: Multiplier,
    pub fire_rate_mult: Multiplier,
}
```

- `merge(&mut self, other)` 疊加多個零件的修改值
- `compute_effective(base, tuning)` 產生最終 `EffectiveStats`
- 控制減免：`R = product(1 + r_i) - 1`，倍率 = `max(0, 1 - R)`

### 第 3 層：EffectiveStats（`game/stats/effective.rs`）

戰鬥中使用的唯讀計算數值。每個配裝快取一份。

---

## SQLite 持久化 — `storage/sqlite_repo.rs`

### 資料表

| 資料表 | 欄位 | 用途 |
|--------|------|------|
| `parts` | `id, slot, kind, spec_json` | 所有自訂零件（JSON blob） |
| `builds` | `id, top_id, weapon_id, shaft_id, chassis_id, screw_id, note` | 自訂配裝 |
| `maps` | `id, name, arena_radius, placements_json` | 自訂地圖 |

### 主要同步方法（設計插件使用）

```rust
// 零件
repo.save_part_sync(rt, slot, kind, id, spec_json) -> Result<(), String>
repo.load_parts_by_slot_sync(rt, slot) -> Result<Vec<(id, kind, json)>, String>
repo.delete_part_sync(rt, id) -> Result<(), String>

// 配裝
repo.save_build_sync(rt, build: &Build) -> Result<(), String>
repo.load_all_builds_sync(rt) -> Result<Vec<...>, String>
repo.delete_build_sync(rt, id) -> Result<(), String>

// 地圖
repo.save_map_sync(rt, id, name, arena_radius, placements_json) -> Result<(), String>
repo.load_all_maps_sync(rt) -> Result<Vec<...>, String>
repo.delete_map_sync(rt, id) -> Result<(), String>
```

---

## 資產慣例

零件 ID 自動決定檔案路徑：

| 零件類型 | 資產路徑 |
|---------|---------|
| 輪盤 | `assets/tops/{id}.png` |
| 武器 | `assets/weapons/{id}.png` |
| 投射物 | `assets/projectiles/{id}_projectile.png` |
| 軸 | `assets/shafts/{id}.png` |
| 底盤 | `assets/chassis/{id}.png` |
| 螺絲 | `assets/screws/{id}.png` |

圖片遺失 → 程序性備用網格（遊戲照常運行）。

設計工坊的「設定圖片」按鈕使用 `rfd::FileDialog` 選擇 PNG，並複製到對應的 `assets/{slot}/` 目錄，使用零件預先產生的 ID 命名。
