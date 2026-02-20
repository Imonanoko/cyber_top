# 程式碼地圖

> 快速定位程式碼的參考文件。每個條目 = 檔案路徑 + 內容說明。

---

## 目錄結構

```
src/
├── main.rs                          # 應用程式入口、視窗 1200×900、Plugin 註冊
├── assets_map.rs                    # GameAssets Resource（精靈圖 + 音效 handle）
├── config/
│   └── tuning.rs                    # Tuning Resource，F5 熱重載，tuning.ron
├── game/
│   ├── components.rs                # GamePhase 列舉、Top/Projectile 標記、區域/Boost 組件
│   ├── events.rs                    # GameEvent、CollisionMessage（Message 型別）
│   ├── collision.rs                 # detect_collisions（陀螺間、陀螺-牆、投射物-陀螺、障礙物）
│   ├── combat.rs                    # 傷害/控制套用、近戰偵測、遠程射擊
│   ├── physics.rs                   # 物理積分、旋轉消耗、控制/近戰計時
│   ├── hooks.rs                     # 特性螺絲鉤子管線（v0：空操作）
│   ├── map.rs                       # MapSpec、MapPlacement、MapItem、GRID_CELL_SIZE
│   ├── parts/
│   │   ├── mod.rs                   # Build struct（已解析的輪盤+武器+零件）
│   │   ├── registry.rs              # PartRegistry、BuildRef、resolve_build()、maps HashMap
│   │   ├── weapon_wheel.rs          # WeaponWheelSpec、MeleeSpec、RangedSpec
│   │   ├── shaft.rs                 # ShaftSpec（穩定性、旋轉效率）
│   │   ├── chassis.rs               # ChassisSpec（速度/加速度/半徑修改）
│   │   └── trait_screw.rs           # TraitScrewSpec、TraitPassive、鉤子
│   ├── stats/
│   │   ├── types.rs                 # 新型別（SpinHp、Radius 等）、列舉（WeaponKind、PartSlot、ControlEffect）
│   │   ├── base.rs                  # BaseStats（不可變輪盤參數）
│   │   ├── effective.rs             # EffectiveStats（Base + 修改值計算結果）
│   │   └── modifier.rs              # StatModifier、ModifierSet、疊加邏輯
│   ├── status/
│   │   └── effect.rs                # （保留空檔，StatusEffect 系統已移除）
│   └── arena/
│       ├── circle.rs                # 牆壁反彈（固定 wall_damage_k，不按速度縮放）
│       └── obstacle.rs              # 靜態障礙物反彈 + 投射物生成/清理
├── storage/
│   ├── repo.rs                      # （保留空檔，BuildRepository trait 已移除）
│   └── sqlite_repo.rs               # SqliteRepo：零件/配裝/地圖的 async+sync CRUD
└── plugins/
    ├── game_plugin.rs               # FixedUpdate 管線、競技場設置、區域系統、瞄準、發射
    ├── map_design_plugin.rs         # 地圖清單（DesignMapHub）+ 格子編輯器（EditMap）
    ├── menu_plugin.rs               # 主選單、Selection、地圖選擇、配裝選擇
    ├── design_plugin.rs             # 設計工坊（所有編輯器、管理、配裝組合）
    ├── storage_plugin.rs            # StoragePlugin、TokioRuntime Resource
    └── ui_plugin.rs                 # 戰鬥 HUD（HP、有效速度、有效武器傷害）
```

---

## Plugin → GamePhase 負責範圍

| Plugin | 負責的 Phase |
|--------|-------------|
| `MenuPlugin` | MainMenu, Selection, PickMap, PickTop, GameOver |
| `GamePlugin` | Aiming, Battle |
| `DesignPlugin` | DesignHub, EditTop, EditWeapon, EditShaft, EditChassis, EditScrew, ManageParts, AssembleBuild, PickDesignPart |
| `MapDesignPlugin` | DesignMapHub, EditMap |

---

## 關鍵 Resource 位置

| Resource | 檔案 | 用途 |
|----------|------|------|
| `Tuning` | `config/tuning.rs` | 所有遊戲常數，可熱重載 |
| `PartRegistry` | `game/parts/registry.rs` | 記憶體中的所有零件 + 配裝 + 地圖 |
| `GameSelection` | `plugins/menu_plugin.rs` | 當前模式、地圖、P1/P2 配裝 ID |
| `PickingFor` | `plugins/menu_plugin.rs` | 選擇畫面中是哪位玩家（1 或 2） |
| `DesignState` | `plugins/design_plugin.rs` | 工坊狀態（正在編輯的 ID、配裝槽位、錯誤訊息） |
| `MapDesignState` | `plugins/map_design_plugin.rs` | 地圖編輯器狀態（當前規格、選中工具、刪除錯誤） |
| `GameAssets` | `assets_map.rs` | 精靈圖 + 音效 handle |
| `ProjectileAssets` | `game/components.rs` | 投射物網格/材質/精靈圖 |
| `ArenaRadius` | `game/components.rs` | 當前競技場半徑（可能與 tuning 預設不同） |
| `SqliteRepo` | `storage/sqlite_repo.rs` | 資料庫存取（零件、配裝、地圖） |
| `TokioRuntime` | `plugins/storage_plugin.rs` | async 橋接 |
