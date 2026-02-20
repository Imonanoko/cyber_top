# Cyber Top — 架構說明

## 概覽

Cyber Top 是一款用 **Rust + Bevy 0.18** 開發的 2D 戰鬥陀螺遊戲。
兩顆陀螺被發射進圓形競技場，發射後所有移動皆由物理驅動（碰撞、牆壁反彈）。最後旋轉 HP 大於 0 的陀螺獲勝。

---

## 遊戲流程（狀態機）

```
MainMenu → Selection → PickMap / PickTop → Aiming → Battle → GameOver → MainMenu
                 ↕                                                ↕
          DesignHub → ManageParts → EditTop / EditWeapon / ...        DesignMapHub → EditMap
                                 → AssembleBuild → PickDesignPart
```

### GamePhase 狀態說明

**主遊戲流程：**
- **MainMenu**：標題畫面，含「開始遊戲」、「設計地圖」、「設計陀螺」按鈕
- **Selection**：選擇模式（PvP / PvAI）、地圖、P1/P2 配裝
- **PickMap**：獨立地圖選擇畫面，顯示卡片預覽
- **PickTop**：配裝選擇畫面。透過 `PickingFor` Resource 區分 P1/P2
- **Aiming**：玩家旋轉發射方向（方向鍵 + 空白鍵）。P2：A/D + Enter。AI 自動隨機確認
- **Battle**：物理驅動的戰鬥。FixedUpdate 系統運行
- **GameOver**：勝利畫面。ESC / Enter 返回主選單

**設計工坊流程：**
- **DesignHub**：入口 — 建立零件、管理零件
- **ManageParts**：列出所有自訂零件與配裝，可編輯 / 刪除
- **EditTop**：陀螺本體編輯（旋轉 HP、半徑、速度、加速度、控制減免）
- **EditWeapon / EditShaft / EditChassis / EditScrew**：零件編輯器（文字輸入、圖片指定、武器類型選擇）
- **AssembleBuild**：組合配裝（選擇每個槽位的零件）
- **PickDesignPart**：組合配裝時選擇特定槽位零件

### 遊戲模式
- **PvAI**：玩家 vs AI。AI 從可用配裝中隨機選擇
- **PvP**：玩家 vs 玩家。兩位玩家各自選擇配裝與瞄準方向

---

## Plugin 架構

| Plugin | 檔案 | 負責範圍 |
|--------|------|---------|
| `GamePlugin` | `plugins/game_plugin.rs` | FixedUpdate 排程、競技場/區域設置、區域系統、瞄準、發射 |
| `MenuPlugin` | `plugins/menu_plugin.rs` | 主選單、選擇畫面、地圖/配裝選擇、遊戲結束畫面 |
| `DesignPlugin` | `plugins/design_plugin.rs` | 設計工坊：零件編輯器、配裝組合、零件管理 |
| `MapDesignPlugin` | `plugins/map_design_plugin.rs` | 地圖清單（DesignMapHub）與格子編輯器（EditMap） |
| `UiPlugin` | `plugins/ui_plugin.rs` | 戰鬥 HUD（HP、有效速度、有效武器傷害） |
| `StoragePlugin` | `plugins/storage_plugin.rs` | SQLite/SQLx 初始化、TokioRuntime Resource |

---

## FixedUpdate 管線（僅 Battle 階段）

SystemSets 嚴格鏈式順序：

```
1. PhysicsSet（鏈式）：
   speed_boost_system → speed_boost_tick → damage_boost_system →
   gravity_device_system → integrate_physics → integrate_projectiles →
   spin_drain → tick_control_state → tick_melee_trackers →
   wall_reflection → static_obstacle_bounce

2. CollisionDetectSet：
   detect_collisions

3. EventGenerateSet（鏈式）：
   generate_collision_damage → detect_melee_hits → fire_ranged_weapons

4. HookProcessSet：
   process_hooks（v0 空操作）

5. EventApplySet（鏈式）：
   apply_damage_events → apply_control_events → resolve_top_collisions →
   spawn_projectiles

6. CleanupSet（鏈式）：
   despawn_projectiles_outside_arena → cleanup_ttl → handle_despawn_events → play_sound_effects
```

---

## 關鍵 Resource

| Resource | 說明 |
|----------|------|
| `Tuning` | 所有可調參數，從 `tuning.ron` 載入，F5 熱重載 |
| `PartRegistry` | 資料驅動的零件預設（陀螺、武器、軸、底盤、螺絲、配裝、地圖） |
| `GameSelection` | 當前模式、地圖、P1/P2 配裝 ID |
| `PickingFor` | 選擇畫面中是哪位玩家（1 或 2） |
| `ProjectileAssets` | 投射物網格/材質 + 每個武器的精靈圖 handle |
| `GameAssets` | 所有精靈圖 handle + 音效 handle，在啟動時載入 |
| `DesignState` | 設計工坊的當前狀態（正在編輯的零件 ID、配裝組合槽位等） |
| `SqliteRepo` | SQLite 資料庫存取（零件、配裝、地圖） |
| `TokioRuntime` | async 轉 sync 橋接的 Tokio runtime |
| `ArenaRadius` | 當前競技場半徑（自訂地圖時可能與 tuning 預設不同） |

---

## 配裝系統

玩家選擇**配裝**（非個別零件）。一套配裝 = 陀螺本體 + 武器 + 軸 + 底盤 + 特性螺絲。

### BuildRef（記憶體中）
`PartRegistry.builds` 存放含零件 ID 的 `BuildRef`。在競技場設置時透過 `resolve_build()` 解析為完整 `Build` struct。

### 預設配裝
| Build ID | 名稱 | 陀螺 | 武器 |
|----------|------|------|------|
| `default_blade` | Standard Blade Top | default_top | basic_blade（近戰） |
| `default_blaster` | Standard Blaster Top | default_top | basic_blaster（遠程） |

### 自訂配裝
透過設計工坊 → 組合配裝建立。儲存至 SQLite `builds` 表，啟動時透過 `merge_custom_builds()` 載入至 `PartRegistry.builds`。

---

## Entity 生命週期

- **InGame 標記**：所有遊戲場次實體以 `InGame` 組件標記
- **清理**：`OnEnter(MainMenu)` 時 `cleanup_game` 清除所有 `InGame` 實體
- **投射物**：在命中、存活時間到期或離開競技場邊界時清除

---

## 訊息系統（Bevy B0002 workaround）

- `CollisionMessage`：陀螺間碰撞資料（獨立型別以避免 Res/ResMut 衝突）
- `GameEvent`：DealDamage、ApplyControl、SpawnProjectile（含 `weapon_id` 用於精靈查找）、DespawnEntity

---

## 數值架構（3 層）

1. `BaseStats` — 每個陀螺不可變的基底參數
2. `ModifierSet` — 來自零件 + 被動特性 + 狀態的修改值
3. `EffectiveStats` — Base + 修改值套用後的結果；快取，在裝備變更時重新計算

---

## 資產系統

### 慣例式載入
- 陀螺 ID `"default_top"` → `assets/tops/default_top.png`
- 武器 ID `"basic_blade"` → `assets/weapons/basic_blade.png`
- 遠程武器 `"basic_blaster"` → `assets/projectiles/basic_blaster_projectile.png`
- 可透過 `BaseStats` / `WeaponWheelSpec` 中的選擇性 `sprite_path` 欄位覆蓋

### 備用策略
- **圖片遺失** → 以備用顏色生成程序性網格（遊戲照常運行）
- **音效遺失** → 靜音（Bevy 內建處理）

### 圖片規格（PNG，RGBA）

| 資產類型 | 建議尺寸 | 備注 |
|---------|---------|------|
| 陀螺本體 | 128×128 px | 朝右（+X方向）。在遊戲中縮放至世界半徑 |
| 武器（近戰） | 128×32 px | 寬度 = 刀刃長度，高度 = 厚度 |
| 武器（遠程） | 64×32 px | 同上 |
| 投射物 | 32×32 px | 透過 Transform.scale 縮放 |
| 軸 / 底盤 / 螺絲 | 128×128 px | 僅在 UI 預覽中顯示 |
| UI 圖示 | 32×32 px | 提供普通版 + hover 版（`{name}.png`，`{name}_hover.png`） |

### 資產目錄結構
```
assets/
  tops/           # {top_id}.png
  weapons/        # {weapon_id}.png
  projectiles/    # {weapon_id}_projectile.png
  shafts/         # {shaft_id}.png
  chassis/        # {chassis_id}.png
  screws/         # {screw_id}.png
  ui/             # edit.png, delete.png, + hover 版本
  audio/sfx/      # launch.ogg, collision_top.ogg 等
  obstacles/      # obstacle.png, gravity_device.png, speed_boost.png, damage_boost.png
```

---

## 地圖設計系統

### 地圖資料模型（`src/game/map.rs`）
- `MapSpec { id, name, arena_radius, placements: Vec<MapPlacement> }`
- `MapPlacement { grid_x, grid_y, item: MapItem }`
- `MapItem`：`Obstacle | GravityDevice | SpeedBoost | DamageBoost`
- 格子大小 = 0.5 世界單位；世界位置 = `(grid_x × 0.5, grid_y × 0.5)`
- 放置有效條件：`dist_from_center + 0.25 < arena_radius`

### 儲存
- SQLite `maps` 表：`id TEXT PK, name TEXT, arena_radius REAL, placements_json TEXT`
- CRUD：`save_map_sync`、`load_all_maps_sync`、`delete_map_sync`（`SqliteRepo` 中）
- 啟動時載入至 `PartRegistry.maps: HashMap<String, MapSpec>`
- 內建：`"default_arena"`（半徑 12.0，無放置物）始終存在

### 競技場設置（`setup_arena`）
- 從 `registry.maps[selection.map_id]` 查找 `MapSpec`
- 使用 `map.arena_radius`（覆蓋 `tuning.arena_radius`）
- 為每個放置物生成實體（詳見 `docs/zh/map-items.md`）

---

## Bevy 0.18 API 注意事項

- `ChildSpawnerCommands`（非 `ChildBuilder`）— `with_children` 閉包的參數型別
- `despawn()` 預設遞歸清除 — `despawn_recursive()` 已移除
- `BorderRadius` 是 `Node` 上的欄位，非獨立 Component
- Bundle tuple 最多約 15 個元素 — 超過時需巢狀 tuple
- 使用 `MessageWriter<T>` / `MessageReader<T>` 處理遊戲事件（非 `Events<T>`）
- Query 衝突（B0001）：必須使用 `Without<T>` 證明不相交；`Changed<T>` 無效
