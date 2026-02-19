# Cyber Top — 遊戲設計規格

> 陀螺在無摩擦的圓形競技場中戰鬥。

---

## 核心玩法

### 陀螺參數
- **旋轉（RPM）= HP**：旋轉降至 0 → 淘汰。
- **大小（半徑）**：影響自然旋轉消耗與受到傷害倍率。
- **移動速度**
- **控制減免**（`r`）：乘法疊加 `R = Π(1 + r_i) - 1`，有效倍率 `m = max(0, 1 - R)`。

### 戰鬥節奏（設計意圖）
- 自然旋轉消耗：**很小**。
- 障礙物 / 牆壁碰撞旋轉消耗：**很小**。
- 差異化來自武器 / 零件 / 特性。

---

## 物理模型（v0：無摩擦）

- **無摩擦**：速度只透過輸入、碰撞或外力改變。
- **圓形競技場**，牆壁會反彈。
- `wall_bounce_damping` ≈ 1.0（可在 tuning 中設定）。
- 每次更新與牆壁反彈後，速度被限制在 `max_speed` 以下。

---

## 裝備系統（4 個零件槽）

| 槽位 | 功能 |
|------|------|
| **武器輪** | 武器類型與攻擊規格 |
| **軸** | 穩定性、旋轉效率 |
| **底盤** | 移動速度、加速度 |
| **特性螺絲** | 被動加成 + 事件鉤子 |

### 軸的數值（v0）
- `stability`：降低碰撞位移
- `spin_efficiency`：降低閒置旋轉消耗

### 特性螺絲鉤子
- `on_hit`：附加負面效果
- `on_tick`：生成障礙物
- `on_wall_collision`：額外牆壁傷害
- `on_fire_projectile`：射速 / 散射調整

---

## 傷害模型

### 統一入口
所有傷害以事件處理：`DealDamage { src, dst, amount, kind }`
- `kind`：`Collision | Melee | Projectile | Wall | Obstacle`

### 結算順序（每個 DealDamage）
1. `amount *= src_damage_out_mult`（來源輸出倍率）
2. `amount *= dst_damage_in_mult`（目標承受倍率）
3. `amount = clamp(amount, 0, +∞)`
4. `dst.spin_hp = max(0, spin_hp - amount)`

### 近戰傷害
`DealDamage { kind: Melee, amount: base_damage * melee_damage_scale }`
- `melee_damage_scale = weapon_dmg_out_mult * hit_speed_scale`
- `hit_speed_scale = 1 + k * rel_speed`（k 來自 tuning）

### 投射物傷害
`DealDamage { kind: Projectile, amount: projectile_damage_base }`
- 投射物在命中或離開競技場邊界時清除。

### 碰撞傷害
`collision_damage = tuning.collision_damage_k * rel_speed`
- 牆壁：`wall_damage_k * rel_speed`（預設 ≈ 0）
- 障礙物：`ObstacleSpec.damage_on_hit`

### 大小 → 承受傷害
`dst_damage_in_mult = 1 + size_damage_k * (dst.radius - size_radius_ref)`

---

## 武器系統

### 類型：`Melee（近戰） | Ranged（遠程） | （未來）Hybrid`

### 瞄準模式
- `FollowSpin`：方向 = 陀螺旋轉角度
- `SeekNearestTarget`：方向 = 朝向最近目標（未來實作）

### 遠程規格
- 射速、連發 / 散射、散射角度、擊退距離
- 投射物半徑、控制持續時間、射程 / 存活時間、瞄準模式

### 近戰規格
- 基礎傷害、命中冷卻、每轉最大命中次數
- 判定框（半徑 / 角度）、命中控制效果（眩暈 / 緩速）

### 控制效果
- `Stun { duration }`（眩暈）、`Slow { duration, ratio }`（緩速）、`Knockback { distance }`（擊退）
- **擊退是控制效果** → 受控制減免倍率 `m` 影響
- `effective_duration = base_duration * m`
- `effective_distance = distance * m`

---

## 障礙物系統（TTL 基礎）

### 實例欄位
- `id`、`owner`（選填）、`spawn_time`、`expires_at`
- `shape`：圓形（可擴展為矩形 / 多邊形）
- `collision_behavior`：`Solid | DamageOnHit(amount) | ApplyControlOnHit(control)`

### 清理：每個 tick，若 `now >= expires_at` → `DespawnEntity`

---

## 新型別（單位型別）
- `SpinHp(f32)` — `sub_clamped`、finite 檢查
- `Radius(f32)`
- `MetersPerSec(f32)`
- `Seconds(f32)` — `dec(dt)` → `max(0)`
- `Multiplier(f32)`
- `AngleRad(f32)` — 每 tick `rem_euclid(TAU)`

---

## 數值安全

### f32（連續數量）
- 每次更新後限制；`debug_assert!(is_finite())`
- `SpinHp`：`[0, spin_hp_max]`
- `Multiplier`：`[0.0, 10.0]`
- `MoveSpeed`：`[0, max_speed]`

### u64（離散計數器）
- `tick_index`、`event_seq`：`checked_add`（溢位 = panic = bug）
- UI/統計計數器：`saturating_add`

### 必要限制清單
- `wall_bounce_damping`：`[0.0, 1.0]`
- `damage_taken_mult`：`[0.0, 10.0]`
- `fire_rate_mult`：`[0.1, 10.0]`
- `stability`：`[0, stability_max]`
- 旋轉消耗：非負、有上限

---

## 資料庫 Schema（SQLite，JSON blob + balance_version）

### 資料表
- `tops`：id, base_stats_json, skin_id, balance_version
- `parts`：id, slot, kind, spec_json, balance_version
- `builds`：id, top_id, weapon_id, shaft_id, chassis_id, screw_id, note
- `maps`：id, name, arena_radius, placements_json
