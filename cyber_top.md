# Cyber Top

> **目的**：這份文件是開發的規格書。  
> 覆蓋你目前定案的全部內容：核心玩法、參數與型別、四部件裝備、事件/結算管線、無摩擦物理與反射、武器/投射物瞄準模式、障礙物 TTL、可調參數 tuning、SQLite + SQLx 資料儲存與快取、以及整體模組架構與里程碑。  
> **範圍**：單機桌面版（離線可玩）。

---

## 0. 技術目標與非目標

### 0.1 技術目標
- Rust 實作 desktop app。
- 以 **struct / enum** 定義所有核心概念與參數，讓新增功能時由編譯器協助補齊分支。
- 遊戲核心採「**事件驅動結算** + **資料驅動 spec**」架構：部件/特性/障礙物/武器以 spec + modifiers + hooks 描述。
- 單機資料儲存：**SQLite + SQLx**，並支援「預運算 EffectiveStats 快取」。
- 常用調參集中於單一 tuning 檔，支援「按鍵 reload」（可選 dev file watch）。


### 0.2 遊戲框架（定案：Bevy）

- 本專案使用 **Bevy** 作為 Rust 遊戲框架（ECS / Schedules / Events / Assets / Input / Render）。
- 遊戲邏輯必須以 Bevy ECS 實作：
  - `Top / Projectile / Obstacle` 為 ECS Entity + Components
  - `tuning / DB pool / asset map / runtime config` 為 Resources
  - 事件系統使用 Bevy `EventWriter` / `EventReader`
- 遊戲 tick 使用固定步長：
  - 所有物理、碰撞、事件生成、hook、事件套用、TTL 清理必須在 `FixedUpdate` 執行
  - 輸入讀取與 UI/Debug 面板更新在 `Update` 執行

---

## 1. 核心玩法定義

### 1.1 基礎參數（核心）
陀螺（Top）的基本參數如下：

- **轉速（Spin / RPM） = 生命值（HP）**
  - 轉速歸零即出局/停止。
- **大小（Size / Radius）**
  - 影響自然轉速消耗（越大阻力越大 → 消耗可更高）。
  - 影響受傷倍率（越大可能更容易受撞擊 → 受傷可更多）。
- **移動速度（Move Speed）**
- **控制減免（Control Reduction）**（以比例 `r` 表示）
  - 正數：縮短控制時間（stun/slow 等）。
  - 負數：增加控制時間（更容易被控）。

> **控制減免（Control Reduction）疊加規則（乘法疊加）**：
> - 每個控制減免效果給一個比例 `r_i`（例如 20% → `0.2`）。
> - 合併後的控制減免比例 `R = Π(1 + r_i) - 1`。
> - 對所有控制效果，套用倍率 `m = 1 - R`。
> - 若 `m < 0`，則視為 `m = 0`。

### 1.2 目標戰鬥節奏（你已定案）
- 希望「多一點戰鬥」，因此：
  - **自然轉速消耗要小**。
  - **碰撞障礙物的轉速消耗也要小**（避免地形 RNG 太重）。
  - 互撞仍需有差距與決勝性，但可透過裝備/武器/特性放大差異。

---

## 2. 物理與運動模型（v0 定案：無摩擦地板）

### 2.1 無摩擦地板
- 地板 **沒有摩擦力**：速度向量不會自然衰減。
- 速度改變只來源於：
  1. 玩家輸入（推進/轉向）對速度施加 Δv
  2. 碰撞（牆/障礙/陀螺）反射或脈衝
  3. 特性/地板效果提供外力（未來）

### 2.2 圓形場地 + 反射
- 場地為圓形 arena。
- 碰撞圓形邊界時，速度向量做反射（reflect）。
- 可配置 **牆面彈性/阻尼**：`wall_bounce_damping`（接近 1.0，以符合「多戰鬥」節奏）。

### 2.3 物理參數可調（由 tuning 管理）
- tick dt（例如 1/60）
- 最大速度/加速度
- 牆反射阻尼
- 碰撞結算係數（restitution 等，如需要）

---

## 3. 遊戲迴圈（Tick Pipeline）


### 3.1 Bevy Schedule 對應（必須遵守）

- `Startup`：
  - 載入/建立 `tuning`、初始化 SQLite、跑 migrations、載入或建立預設 `build`、建立 `EffectiveStats` 快取、載入 skins/assets 對照表。
- `Update`：
  1. 讀 input（玩家/AI）
  2. 更新 UI/Debug（配裝、tuning reload、顯示資料）
  3. 將輸入寫入 `Intent` components/resources（只寫意圖，不改物理狀態）
- `FixedUpdate`（固定步長 tick，唯一改動戰鬥狀態的地方）：
  1. Intent → 施加加速度/轉向（改 velocity）
  2. 物理更新（位置/速度/旋轉角）
  3. 碰撞偵測（Top–Top / Top–Wall / Top–Obstacle / Projectile–* / Melee hitbox–Top）
  4. 產生事件（Event）
  5. Hook/Modifier 管線（部件 hooks → 狀態 hooks → 場地 hooks → 全域規則）
  6. 套用事件（spin/控制/狀態效果）
  7. TTL 清理（Obstacle / Projectile）


固定步長 tick（例如 60 FPS）：

1. 讀 input（玩家/AI）
2. 更新意圖（加速、轉向、技能、攻擊）
3. 物理更新（速度/位置/旋轉角）
4. 碰撞偵測  
   - Top–Top  
   - Top–Wall（圓邊界）  
   - Top–Obstacle  
   - Projectile–Top / Projectile–Obstacle  
   - Melee weapon hitbox–Top（如有）
5. 產生事件（Event）
6. 事件經 hooks/modifiers 管線處理（部件/狀態/地板/障礙物）
7. 套用事件到狀態（spin/控制/狀態效果）
8. 清理 TTL 到期 entity（Obstacle / Projectile）
9. 渲染

---

## 4. Entity 與核心資料結構

### 4.1 Entity 類型
- `Top`
- `Projectile`
- `Obstacle`
- （未來）`FloorZone`（地板效果區域）

### 4.2 Top 需要的 runtime state（最小）
- 位置/速度向量
- 旋轉角（用於「跟著陀螺旋轉」的武器/投射物方向）
- `spin_hp_current`（轉速當 HP）
- 控制狀態（stun/slow 等剩餘時間）
- 狀態效果列表（buff/debuff instances）
- 裝備 build id（或直接持有四部件 id）
- **EffectiveStats（預運算快取）**：戰鬥 tick 直接讀，不在 tick 內重算

---

## 5. Stats 架構（你希望「簡單計算」的定案）

### 5.1 三層 Stats 模型（可快取）
- `BaseStats`：基礎參數（不直接被改寫）
- `ModifierSet`：部件 + 常駐特性 + 狀態 + 場地
- `EffectiveStats`：Base 套用 modifiers 後的結果（**換裝/進戰時重算，戰鬥 tick 只讀**）

> 例如：特性螺絲 +20% 轉速不直接改 BaseStats，而是透過 modifier 影響 EffectiveStats，並把 EffectiveStats 快取，達成簡化。

### 5.2 Modifier 疊加規則（必須明確）
- 每個欄位定義：
  - **add**（加法）
  - **mul**（乘法）
  - **clamp**（例如 control_reduction 範圍）
- 建議把 modifier 結構拆為：
  - `StatAdd`（可為 0）
  - `StatMul`（預設 1）
  - `StatClamp`（可選）

---

## 6. 多部件裝備系統（本期核心）

你定案拆 4 部份：

1. **Weapon Wheel（武器輪盤）**：決定武器型態與攻擊 spec  
2. **Shaft（中軸）**：提供穩定性/耗轉效率等核心手感參數  
3. **Chassis（底盤）**：影響移動/加速度/（可選）碰撞半徑  
4. **Trait Screw（特性螺絲）**：常駐 buff 與事件 hook（命中 debuff、放置地形等）

### 6.1 Shaft（中軸）參數定義（建議採用）
- `stability`：降低碰撞造成的偏移/失控（影響碰撞後速度/方向擾動）
- `spin_efficiency`：降低自然耗轉（spin drain）
- （可選）`recoil_absorb`：遠程射擊反作用的吸收

> v0 必做：`stability + spin_efficiency`

### 6.2 Trait Screw 的事件能力（hook）
Trait screw 不只改數值，還能訂閱事件，例如：
- `on_hit`：命中附加 debuff
- `on_tick`：放置障礙物/地形
- `on_wall_collision`：撞牆扣血更多（對自己或對敵，依 spec）
- `on_fire_projectile`：射速提升/散射調整

---

## 7. 武器/攻擊系統

## 7A. 傷害模型與計算規格（必做）

### 7A.1 傷害事件資料（統一入口）

- 所有傷害必須以事件表示，不允許直接修改 `SpinHp`。
- 使用事件：`DealDamage { src, dst, amount, kind, tags }`
  - `amount`：基礎傷害值（未套用防護/倍率前）
  - `kind`：`Collision | Melee | Projectile | Wall | Obstacle`
  - `tags`：字串/enum 標籤集合（用於 hooks，例如 `wall_hit`, `crit`, `weapon:<id>`）

### 7A.2 傷害結算順序（固定）

對單一 `DealDamage` 事件，依序套用：

1. `amount = amount * src_damage_out_mult`（來源輸出倍率；預設 1）
2. `amount = amount * dst_damage_in_mult`（目標受傷倍率；預設 1；可由 size/狀態/特性影響）
3. `amount = clamp(amount, 0, +∞)`（不得為負）
4. `dst.spin_hp_current = max(0, dst.spin_hp_current - amount)`

> `src_damage_out_mult`、`dst_damage_in_mult` 由 `EffectiveStats` + `StatusEffectInstances` + hooks 決定。  
> 這裡只定義順序；具體倍率來源見 7A.3 與 7A.4。

### 7A.3 近戰（Melee）傷害公式（武器輪盤）

近戰命中時必須生成：

- `DealDamage { kind: Melee, amount: melee_damage_base * melee_damage_scale, ... }`

其中：
- `melee_damage_base`：來自 `MeleeSpec.base_damage`
- `melee_damage_scale`：可選縮放（預設 1），由以下項相乘：
  - `src.weapon_damage_out_mult`（來源武器輸出倍率；由部件/狀態提供）
  - `hit_speed_scale`（可選；預設 1；若啟用則與相對速度相關）
    - `hit_speed_scale = 1 + k * rel_speed`（k 由 tuning；若不啟用則固定 1）

命中限制：
- `MeleeSpec.hit_cooldown`：同一目標命中冷卻（秒）
- `MeleeSpec.max_hits_per_rotation`：每轉最多命中次數（可選）

### 7A.4 遠程（Projectile）傷害公式（投射物）

投射物命中時必須生成：

- `DealDamage { kind: Projectile, amount: projectile_damage_base, ... }`
- `ApplyControl { ... }`（若投射物帶控制）
- （可選）`Knockback`（作為控制效果，並套用控制減免倍率）

其中：
- `projectile_damage_base`：來自 `RangedSpec.projectile_damage`
- 命中後投射物消失（預設），除非 spec 明確定義穿透/反彈等。

### 7A.5 碰撞（Top–Top / Top–Obstacle / Top–Wall）傷害公式

碰撞產生 `Collision { impulse, normal }` 後，必須在 `EventGenerateSet` 轉換成 `DealDamage`（以及可選控制）。

定義：
- `rel_speed`：碰撞瞬間相對速度（標量）
- `impulse_mag`：碰撞脈衝大小（若有物理計算可用；否則用 `rel_speed` 近似）

建議公式（v0 最小可行）：
- `collision_damage = tuning.collision_damage_k * rel_speed`

然後生成：
- `DealDamage { kind: Collision, amount: collision_damage, tags: ["collision"] }`

牆/障礙物碰撞傷害（若要有）：
- `wall_damage = tuning.wall_damage_k * rel_speed`（可非常小或 0）
- `obstacle_damage = ObstacleSpec.damage_on_hit`（若該障礙物為 DamageOnHit）

> 你已要求「撞牆/障礙耗轉小」，因此 `wall_damage_k` 預設可為 0 或極小；真正差距主要靠武器/部件。

### 7A.6 size（大小）對受傷倍率的影響（定義為受傷倍率）

`size_radius` 影響 `dst_damage_in_mult`：

- `dst_damage_in_mult = 1 + tuning.size_damage_k * (dst.radius - tuning.size_radius_ref)`

規則：
- 若不啟用此功能，`size_damage_k = 0`。
- 若啟用，必須確保最終倍率不得為負（事件結算已 clamp amount >= 0）。

### 7A.7 Trait Screw / Status 的增傷與減傷（以倍率表示）

允許以下效果修改倍率：
- `DamageOutMult(x)`：乘到 `src_damage_out_mult`
- `DamageInMult(x)`：乘到 `dst_damage_in_mult`
- `WallCollisionPenaltyMult(x)`：僅對 `kind: Wall/Collision` 或 `tags` 包含 `wall_hit` 時乘上

所有倍率疊加規則：
- 同類倍率採乘法疊加：`mult = Π(mult_i)`
- 預設倍率為 1

---

### 7.1 武器類型（至少要 enum）
- `Melee`
- `Ranged`
- （未來）`Hybrid`

### 7.2 控制減免套用範圍（定案）

- **擊退（Knockback）屬於控制效果**，同樣套用控制減免倍率 `m`。
- 套用規則：
  - `Stun/Slow`：`effective_duration = base_duration * m`
  - `Knockback(distance)`：`effective_distance = distance * m`

### 7.2 武器方向/瞄準模式（你已定案）
遠程投射物方向（以及未來武器朝向）使用：

- `AimMode::FollowSpin`  
  初期：方向 = 陀螺當下旋轉角（跟著陀螺旋轉）
- `AimMode::SeekNearestTarget`  
  未來：方向 = 指向最近目標（可加 turn_rate 限制）

> 這個 AimMode 必須同時適用於：  
> - 投射物生成初始方向  
> -（未來）武器持續朝向更新（例如每 tick 轉向最近目標）

### 7.3 遠程發射物 spec（你最初列的參數）
Ranged spec 至少包含：
- 射速（rate of fire）
- 發射型態：連發/散射
- 散射角度（spread angle）
- 擊退距離（通常 0）
- 物品大小（projectile radius）
- 控制時間（命中造成控制）
- 有效射程（range / lifetime）
- AimMode（FollowSpin / SeekNearestTarget）

### 7.4 近戰武器 spec（先留欄位）
- base damage
- 命中頻率限制（cooldown / 每轉最多命中次數）
- hitbox（半徑/角度）
- 命中控制（stun/slow）

---

## 8. 障礙物（Obstacle）— 有時效（TTL）定案

你希望障礙物有時效，增加策略性並方便擴充新功能。

### 8.1 Obstacle instance 最小欄位
- `id`
- `owner`（可選）
- `spawn_time`
- `expires_at`（或 ttl）
- `shape`：先 circle（可擴 rect/polygon）
- `collision_behavior`：
  - `Solid`（反射/阻擋）
  - `DamageOnHit(amount)`
  - `ApplyControlOnHit(control)`
  - （未來）`ZoneEffect(...)`（區域性）

### 8.2 TTL 清理
每 tick：
- `if now >= expires_at` → emit `DespawnEntity(obstacle_id)`

> Obstacle spec 建議資料化（DB/檔案），instance 不一定要存 DB（除非 replay）。

---

## 9. 場地與地板

場地設計：
- 圓形場地
- 未來要有設計介面，場地內有不同地板/障礙物
- 地板類型（未來）：
  - 加速帶
  - 吸引地板（代替現實的向中心靠近）
  - 增益地板等

> v0 只需：圓形邊界 + 無摩擦 + 反射。  
> 保留 `FloorZoneSpec`/`ExternalForce` 接口，未來擴充不改核心。

---

## 10. 事件系統（Event-Driven Resolution）

### 10.1 Event enum（最小集合）
- 本節的 Event enum 在 Bevy 中實作為 `GameEvent`（Bevy `Event`），使用 `EventWriter<GameEvent>` / `EventReader<GameEvent>`。
- `Collision { a, b, impulse, normal }`
- `DealDamage { src, dst, amount, kind }`
- `ApplyControl { src, dst, control }`
- `ApplyStatus { src, dst, status }`
- `SpawnProjectile { src, spec }`
- `SpawnObstacle { src, spec, ttl }`
- `DespawnEntity { id }`

### 10.2 Hook/Modifier 管線


### 10.3 Bevy SystemSet 排序（固定順序）

在 `FixedUpdate` 設定以下 SystemSet，並用 `.configure_sets(...)` 明確指定順序：

1. `InputIntentSet`（消化 Intent，寫入加速度/轉向）
2. `PhysicsSet`（integrate velocity/position/angle）
3. `CollisionDetectSet`
4. `EventGenerateSet`
5. `HookProcessSet`
6. `EventApplySet`
7. `CleanupSet`（TTL、despawn）

事件生成後，依序經過：
1. 裝備部件 hooks（4 部件）
2. 狀態效果 hooks（buff/debuff）
3. 場地/地板 hooks（未來）
4. 全域規則（tuning、cap、clamp）

然後才落地修改 Top/Projectile/Obstacle state。

> **關鍵原則**：碰撞/攻擊系統不直接改 HP；只產生事件。  
> 這樣才能乾淨插入 trait screw 的 on_hit、牆增傷等功能。

---

## 11. 視覺（前端顯示圖案

### 11.1 Skin / VisualSpec
- `skin_id`：存在 Top 資料裡（enum 或 string key）
- `VisualSpec`：由 asset 表提供（sprite/svg/mesh、顏色、圖層）

### 11.2 渲染策略（v0）
- 先用 placeholder（幾何形狀 + 顏色）即可
- 後續替換資產不影響 core logic

---

## 12. 可調參數（tuning）— 單檔集中、可 reload

希望常用參數最好在一個檔案方便改，包含轉速消耗等。

### 12.1 tuning 檔位置
- `data_dir()/tuning.ron`（或 json）
- app 內建 default tuning；首次啟動若不存在則複製一份到 data_dir。

### 12.2 tuning 內容（最小必備）
- `dt`
- `arena_radius`
- `wall_bounce_damping`
- `spin_drain_idle_per_sec`（自然耗轉，小）
- `spin_drain_on_wall_hit`（撞牆耗轉，小）
- `spin_drain_on_top_hit`（互撞耗轉，保留勝負）
- `max_speed`
- `input_accel`

### 12.3 reload 策略
- Release：提供按鍵 reload（跨平台、可控）
- Dev：可選 file watch 熱重載（不依賴 CI/CD）

---

## 13. 資料儲存（SQLite + SQLx）— 單機定案

### 13.1 SQLx 與 migrations
- repo 內 `migrations/`
- app 啟動時：
  1. 建 data_dir
  2. 開 pool（`sqlite://abs_path`）
  3. `sqlx::migrate!().run(&pool).await`

### 13.2 SQLx 離線編譯（建議）
- 開發/CI 生成 `sqlx-data.json`（`cargo sqlx prepare`）
- release build 不依賴 runtime DB 驗證

---

## 14. DB Schema（建議：JSON blob + balance_version）

> 遊戲 spec 會常改，建議把 tops/parts/spec 用 json 儲存，降低 migration 成本。

### 14.1 tables
- `tops`
  - `id TEXT PRIMARY KEY`
  - `base_stats_json TEXT NOT NULL`
  - `skin_id TEXT NOT NULL`
  - `balance_version INTEGER NOT NULL`
- `parts`
  - `id TEXT PRIMARY KEY`
  - `slot TEXT NOT NULL`  (weapon_wheel / shaft / chassis / trait_screw)
  - `kind TEXT NOT NULL`
  - `spec_json TEXT NOT NULL`
  - `balance_version INTEGER NOT NULL`
- `builds`
  - `id TEXT PRIMARY KEY`
  - `top_id TEXT NOT NULL`
  - `weapon_id TEXT NOT NULL`
  - `shaft_id TEXT NOT NULL`
  - `chassis_id TEXT NOT NULL`
  - `screw_id TEXT NOT NULL`
  - `note TEXT`
- `effective_cache`
  - `build_id TEXT PRIMARY KEY`
  - `effective_stats_json TEXT NOT NULL`
  - `computed_at INTEGER NOT NULL`
  - `balance_version INTEGER NOT NULL`
  - `hash TEXT NOT NULL`

### 14.2 EffectiveStats 快取策略（你要的「簡單計算」）
- 進入戰鬥/換裝時：
  - load build + parts + tuning_version → compute effective → write `effective_cache`
- 戰鬥 tick：
  - 只讀 `EffectiveStats`（記憶體快取）；不做 DB IO
- `hash`/`balance_version` 用於判斷 cache 是否失效

---

## 15. Rust 型別與接口（Agent 必須照此建模）

> 這裡不是完整程式碼，但足以讓 agent 開檔案、定義 struct/enum/trait，並確保擴充時 compile-time 強制補齊。

### 15.1 Newtypes（單位型別，建議）
- `SpinHp(f32)`
- `Radius(f32)`
- `MetersPerSec(f32)`
- `Seconds(f32)`
- `Multiplier(f32)`
- `AngleRad(f32)`

### 15.2 核心 enums
- `PartSlot = WeaponWheel | Shaft | Chassis | TraitScrew`
- `AimMode = FollowSpin | SeekNearestTarget`
- `WeaponKind = Melee | Ranged | Hybrid`
- `ControlEffect = Stun { duration } | Slow { duration, ratio } | Knockback { distance }`（Knockback 也套用控制減免倍率 `m`）
- `DamageKind = Collision | Melee | Projectile | Wall | Obstacle`

### 15.3 Stats structs
- `BaseStats`
  - `spin_hp_max`
  - `radius`
  - `move_speed`
  - `control_reduction`
- `ShaftStats`
  - `stability`
  - `spin_efficiency`
- `EffectiveStats`
  - `spin_hp_max`
  - `radius`
  - `move_speed`
  - `control_multiplier`（定義：`m = 1 - R`，`R = Π(1 + r_i) - 1`；若 `m < 0` 則 `m = 0`）
  - `spin_drain_idle_per_sec`
  - `spin_drain_on_wall_hit`
  - `spin_drain_on_top_hit`
  - `stability`
  - （可選）`damage_taken_mult`, `fire_rate_mult` 等

### 15.4 Modifier types（必備）
- `StatModifier`（包含 add/mul/clamp）
- `ModifierSet`（可合併多個 modifier，並產出 EffectiveStats）

### 15.5 Part specs（資料驅動）
- `WeaponWheelSpec`（包含 weapon kind、melee/ranged spec、AimMode 等）
- `ShaftSpec`（stability/spin_efficiency）
- `ChassisSpec`（move/accel 等）
- `TraitScrewSpec`（passives + hooks）

### 15.6 Hook trait（事件管線）
- `trait Hook { fn on_event(&self, ctx: &mut HookContext, event: &mut Event, out: &mut Vec<Event>); }`
- `HookContext` 至少包含：
  - `dt`
  - `tuning`
  - entity lookup（讀取 src/dst 的 EffectiveStats、狀態）
  - rng（如需要）

> 任何 trait screw 的複雜行為都應由 hook 完成，而不是在核心碰撞/攻擊系統硬寫 if-else。

---

## 16. 模組/檔案結構（Bevy 專案版）

```
src/
  main.rs               # Bevy App 建立、註冊 plugins/schedules/events/resources
  plugins/
    mod.rs
    game_plugin.rs      # FixedUpdate/Update/Startup 系統註冊與 SystemSet 排序
    ui_plugin.rs        # 配裝/Debug UI（可選）
    storage_plugin.rs   # SQLite/SQLx 初始化、repo resource
  game/
    mod.rs
    components.rs       # Top/Projectile/Obstacle 等 components
    intent.rs           # Input intent components/resources
    tick.rs             # FixedUpdate 系統集合（呼叫各子系統）
    physics.rs          # movement, reflection, collision primitives
    collision.rs        # broad/narrow phase（簡化版本）
    combat.rs           # 事件生成（不直接改狀態）
    events.rs           # Bevy Events 定義（GameEvent 等）
    hooks.rs            # Hook trait + hook dispatcher（部件/狀態/場地）
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
    repo.rs             # BuildRepository trait（Resource）
    sqlite_repo.rs      # SQLx 實作（Resource）
  config/
    tuning.rs           # Tuning load/reload（Resource）
  assets_map.rs         # skin_id → Handle<Image> / VisualSpec 對照（Resource）
migrations/
assets/
  skins/
```


---

## 17. 開發里程碑（Agent 交付順序）

### 17.1 MVP（必做）
1. World + entity storage（Top/Obstacle/Projectile）
2. 基本 tick pipeline（dt）
3. 無摩擦移動 + 圓形牆反射
4. 碰撞偵測（Top–Wall, Top–Top, Top–Obstacle）
5. Event 系統（Collision/DealDamage/ApplyControl/SpawnProjectile/Despawn）
6. Stats：Base → Effective（快取），+ tuning 控制 spin drain（小）
7. 四部件資料模型 + 至少 1~2 組範例部件
8. WeaponWheel：做 1 個簡單 Melee + 1 個簡單 Ranged（FollowSpin）
9. Obstacle TTL：放置障礙物 + 到期消失
10. SQLite + SQLx：
    - migrations
    - parts/tops/builds/effective_cache
    - 進戰時計算 effective 並快取
11. 渲染 placeholder（用 skin_id 顯示不同形狀/顏色）

### 17.2 v0.2（擴充）
- AimMode::SeekNearestTarget
- Trait screw hooks：on_hit debuff、on_tick 放置障礙物
- 地板效果區域（加速/吸引）
- 分身（clone Top entity 規則）
- 技能系統（cooldown + events）

---

## 18. 重要工程約束（避免性能/可維護性陷阱）

- **戰鬥 tick 不做 DB IO**：只讀記憶體中的 EffectiveStats 和 runtime state。
- **所有可調參數進 tuning**：spin drain、反射阻尼、控制倍率規則 等。
- **新增部件/武器/效果必須靠 enum/match 強制補齊**：避免魔法字串分支散落。
- **事件驅動**：避免在 physics/combat 直接改 HP；改用 events → hooks → apply。

---

## 19. 附錄：最小 tuning 範例（示意）
> 具體格式可選 RON/JSON；這裡只列欄位語意。

- `dt = 0.0166667`
- `arena_radius = 12.0`
- `wall_bounce_damping = 0.99`
- `spin_drain_idle_per_sec = 0.2`
- `spin_drain_on_wall_hit = 0.5`
- `spin_drain_on_top_hit = 1.0`
- `max_speed = 8.0`
- `input_accel = 25.0`

---

## 21. 數值安全與溢位防護規範（必讀）

> 目標：避免整數溢位/underflow、避免 `NaN/Inf` 汙染狀態、避免數值飄走導致不可玩。  
> 原則：**連續量用 `f32` + clamp/is_finite**；**離散計數用 `u64` + checked/saturating**；把規則集中在 newtype 方法裡，避免到處散落 `checked_add`。

### 21.1 型別選擇（按風險分類）

#### A) 連續量（建議 `f32`）
適用：`SpinHp`, `Seconds`, `MetersPerSec`, `AngleRad`, `Multiplier` 等。

- 主要風險不是「整數溢位」，而是：
  - 計算出 `NaN` / `Inf`（例如除以 0、倍率連乘爆大）
  - 產生不合理負值（HP < 0、duration < 0）
  - 數值飄走（速度無上限、角度無限增長）

**必做防護：**
- 每次更新後 **clamp** 到合理範圍
  - `SpinHp`：`[0, spin_hp_max]`
  - `Seconds`：`[0, +∞)`（不允許負）
  - `Multiplier`：限制在可玩範圍（例如 `[0.0, 10.0]` 或依 tuning）
  - `MoveSpeed`：`[0, max_speed]`
- Debug/開發模式加入：
  - `debug_assert!(value.is_finite())`  
  - 或在集中入口處遇到非 finite 時直接回傳錯誤/重置為安全值（release 可 log）

> **注意**：`checked_add` 不會解決 `NaN/Inf`，因此對 `f32` 的核心防護是 clamp + is_finite。

#### B) 離散計數（建議 `u64`）
適用：`tick_index`, `event_seq`, `unix_epoch`, `rng_seed_counter` 等。

- 主要風險：長時間運行或 bug 導致溢位。
- 建議 tick 使用 `u64`（避免 `u32` 太快爆）。

**運算策略：**
- 代表「程式 bug、不可接受」：用 `checked_*`
  - 例：`tick = tick.checked_add(1).expect("tick overflow")`
- 代表「不影響核心、寧可不崩」：用 `saturating_*`
  - 例：UI 計數、統計數等

#### C) 整數 HP/傷害（若你未來改用整數）
若 `SpinHp`/傷害用 `u32`：
- 扣血使用 `saturating_sub`（遊戲常見，避免 underflow 變超大數）
- 或 `checked_sub`（更嚴格，溢位視為 bug）

目前規格建議 `SpinHp(f32)`，以倍率/耗轉計算更直覺，並以 clamp 保護。

---

### 21.2 在本遊戲中的「必設 clamp」清單（建議預設值可進 tuning）

- `wall_bounce_damping`：clamp `[0.0, 1.0]`
- `damage_taken_mult`：clamp `[0.0, 10.0]`（避免爆大）
- `fire_rate_mult`：clamp `[0.1, 10.0]`（避免射速趨近 0 或爆炸）
- `move_speed`：clamp `[0, max_speed]`
- `stability`：clamp `[0, stability_max]`（上限可由 tuning 或部件定義）
- `spin drains`（每秒耗轉/碰撞耗轉）：不得為負；且應有上限避免一撞即死

---

### 21.3 Newtype 集中安全算術（避免散落 checked_add）

**要求**：對關鍵數值型別提供集中更新入口，所有寫入狀態都走這些方法。

建議：
- `SpinHp(f32)`
  - `fn add_clamped(self, delta: f32, max: f32) -> Self`
  - `fn sub_clamped(self, delta: f32) -> Self`（不低於 0）
  - 內部 `is_finite` 檢查 + clamp
- `Seconds(f32)`
  - `fn dec(self, dt: f32) -> Self`（`max(0)`）
- `Tick(u64)`
  - `fn next(self) -> Self`（`checked_add(1)`）

> 核心理念：**少量入口**實作安全策略；遊戲邏輯層只呼叫這些方法，不直接做裸加減。

---

### 21.4 遊戲特定風險點與對策

- **倍率連乘**（部件 + buff + 地板 + tuning）：  
  - 必須 clamp multiplier，並在 compute_effective 時做 `is_finite` 檢查。
- **角度無限累積**：  
  - 建議每 tick 做 `angle = angle.rem_euclid(TAU)`（或等價）
- **無摩擦造成速度過大**：  
  - 必須 clamp `speed <= max_speed`；牆反射後也 clamp
- **TTL/剩餘時間**：  
  - 用 `Seconds` 並 `remaining = (remaining - dt).max(0)`；或用 tick TTL 時用 `saturating_sub(1)`。
- **事件數量爆炸**：  
  - 可設每 tick 的事件上限（debug 用 assert），避免 hook 產生無限事件迴圈。

---
