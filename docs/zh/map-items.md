# 地圖物件參考

> 在地圖設計編輯器中放置的每個物件，在戰鬥中都會以實體形式出現。
> 所有物件都標記 `InGame`，戰鬥結束後自動清理。
> 精靈圖放在 `assets/obstacles/`（64×64 RGBA PNG，透過 `python3 gen_assets.py` 重新生成）。

---

## 格子系統

- **格子大小**：每格 0.5 世界單位
- **世界位置**：`(grid_x × 0.5, grid_y × 0.5)`，原點 = 競技場中心
- **有效放置**：格子中心必須至少在競技場邊界內 0.25 單位
- **競技場半徑**：每張地圖可配置，預設 12.0 世界單位

---

## 物件類型

### 障礙物（灰色 X 圖示）

**用途**：靜態牆壁，接觸時阻擋並傷害陀螺。

| 屬性 | 數值 |
|------|------|
| 精靈圖 | `assets/obstacles/obstacle.png` — 深色背景上的灰色 X |
| 視覺大小 | 0.5 × 0.5 世界單位（一個格子） |
| 碰撞半徑 | 0.25 wu（半格 — 精確格子邊界） |
| 編輯器圖章 | 1 × 1 格 |
| 彈跳 | 彈性反射：`v' = v − 2(v·n̂)n̂` |
| 命中傷害 | `tuning.obstacle_damage`（預設 2.0 旋轉 HP） |
| 持久性 | 戰鬥期間永久存在 |

**行為**：當陀螺與障礙物重疊時：
1. `static_obstacle_bounce`（PhysicsSet）將陀螺推出並彈性反射速度。
2. `detect_collisions`（CollisionDetectSet）發出帶 `DamageKind::Obstacle` 的 `DealDamage` 事件。

---

### 重力裝置（紫色圓環圖示）

**用途**：在範圍內持續將陀螺導向自身。

| 屬性 | 數值 |
|------|------|
| 精靈圖 | `assets/obstacles/gravity_device.png` — 紫色同心圓環 |
| 視覺大小 | 6.0 × 6.0 wu（效果半徑直徑） |
| 偵測半徑 | 3.0 wu（從裝置中心） |
| 導向強度 | 3.0（每秒混合方向） |
| 速度保留 | 是 — 只改變方向，不改變速度大小 |
| 編輯器圖章 | 1 × 1 格 |
| 持久性 | 永久 |

**行為**：每個 FixedUpdate tick，對 `device.radius + top_radius` 範圍內的每顆陀螺：
```
blend = clamp(steer_strength × dt, 0.0, 1.0)
new_dir = normalize(current_dir × (1 - blend) + toward_device × blend)
vel = new_dir × speed
```
效果是平滑的連續拉力；若陀螺速度足夠，會繞著裝置旋轉。

---

### 速度提升區（綠色閃電圖示）

**用途**：暫時提升陀螺的有效移動速度。

| 屬性 | 數值 |
|------|------|
| 精靈圖 | `assets/obstacles/speed_boost.png` — 黃綠色閃電 |
| 視覺大小 | 每格 0.5 × 0.5 wu |
| 碰撞半徑 | 每格 0.25 wu（半格） |
| 偵測閾值 | `top_radius + 0.25 ≈ 1.55 wu`（從格子中心） |
| 編輯器圖章 | **2 × 2 格**（每次點擊放置 4 格） |
| 速度倍率 | 1.5× |
| 持續時間 | 最後接觸任意格子後 3.0 秒 |
| 影響組件 | 陀螺上的 `SpeedBoostEffect.multiplier` |

**行為**：`speed_boost_system`（PhysicsSet 第一個）每 tick 檢查重疊。重疊時直接設定 `SpeedBoostEffect { multiplier: 1.5, expires_at: now + 3.0 }`。`integrate_physics` 使用 `eff_vel = vel × multiplier` 計算位置變化。原始 `Velocity` 組件不變；只有位置增量（和視覺旋轉速率）被縮放。

離開格子 3 秒後，`speed_boost_tick` 將 `multiplier` 重置為 1.0。

**覆蓋範圍**：要建立更大的區域，並排放置多個 2×2 圖章。每格獨立觸發加速；多個重疊格子取最大倍率。

**HUD**：`spd:` 顯示有效速度（`vel.length() × multiplier`）— 激活時跳升約 50%。

---

### 傷害提升區（紅色劍圖示）

**用途**：提升碰觸區域的陀螺的武器傷害輸出。

| 屬性 | 數值 |
|------|------|
| 精靈圖 | `assets/obstacles/damage_boost.png` — 深紅底白劍 |
| 視覺大小 | 每格 0.5 × 0.5 wu |
| 碰撞半徑 | 每格 0.25 wu（半格） |
| 偵測閾值 | `top_radius + 0.25 ≈ 1.55 wu`（從格子中心） |
| 編輯器圖章 | **2 × 2 格**（每次點擊放置 4 格） |
| 傷害倍率 | 1.5× 輸出傷害 |
| 持續時間 | 僅在重疊任意格子時有效（離開後立即取消） |
| 影響組件 | 陀螺上的 `DamageBoostActive.multiplier` |

**行為**：`damage_boost_system`（PhysicsSet）每 tick 檢查重疊。重疊時設定 `DamageBoostActive { multiplier: 1.5 }`；不重疊時重置為 1.0。在 `apply_damage_events`（EventApplySet）中套用：
```
final_damage = base_damage × damage_out_mult × dmg_boost.multiplier × damage_in_mult
```
影響該陀螺造成的所有傷害類型：碰撞、近戰、遠程投射物。

**HUD**：`wpn:` 顯示有效武器傷害（`base × out_mult × boost_mult`）— 在區域內跳升約 50%。

---

## 系統執行順序

區域系統在 `PhysicsSet` 開始時執行（`integrate_physics` 之前），確保倍率在同一個 FixedUpdate tick 內套用到移動：

```
speed_boost_system       ← 設定 SpeedBoostEffect.multiplier（入場時記錄 "SpeedBoost ACTIVATED"）
speed_boost_tick         ← 將過期效果重置為 multiplier 1.0
damage_boost_system      ← 設定 DamageBoostActive.multiplier（入場時記錄 "DamageBoost ACTIVATED"）
gravity_device_system    ← 混合速度方向朝向裝置
integrate_physics        ← 套用 eff_vel = vel × speed_mult（激活時每秒記錄一次速度值）
...
```

`DamageBoostActive` 在稍後的 `EventApplySet → apply_damage_events` 套用（每次命中記錄加成後的傷害）。

---

## 設計注意事項

- 所有地圖物件使用 `CollisionRadius(cell_radius)` = `GRID_CELL_SIZE × 0.5` = **0.25 wu**。
- 區域覆蓋範圍由格子數量決定：多放 2×2 圖章可擴大區域。
- `SpeedBoostEffect` 和 `DamageBoostActive` 都是陀螺上的**常駐組件**（生成時 `multiplier: 1.0`）。區域系統直接修改它們 — 無需 `Commands.insert/remove` 的延遲開銷。
- 同類型多個重疊格子：取最大（最高）倍率。
- 精靈圖重新生成：編輯 `gen_assets.py` 後執行 `python3 gen_assets.py` — 不需要 pip 安裝。
