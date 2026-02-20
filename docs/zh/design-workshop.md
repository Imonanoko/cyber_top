# 設計工坊 — `src/plugins/design_plugin.rs`

> 設計輪盤 / 工坊功能的所有程式碼都在同一個檔案中。
> 本文件是該檔案結構的完整參考。

---

## 狀態機

```
MainMenu
  └─ DesignHub
       ├─ EditTop        （建立 / 編輯輪盤）
       ├─ EditWeapon     （建立 / 編輯武器）
       ├─ EditShaft      （建立 / 編輯軸）
       ├─ EditChassis    （建立 / 編輯底盤）
       ├─ EditScrew      （建立 / 編輯特性螺絲）
       └─ ManageParts    （列出所有零件與配裝）
            ├─ Edit*     （編輯現有零件 → return_to_manage=true）
            ├─ AssembleBuild
            │    └─ PickDesignPart  （選擇槽位零件）
            └─ NewBuild → AssembleBuild
```

### Phase → 系統註冊（Plugin::build）

| Phase | OnEnter | OnExit | Update |
|-------|---------|--------|--------|
| DesignHub | `spawn_design_hub` | `despawn::<ScreenRoot>` | `design_hub_system` |
| ManageParts | `spawn_manage_parts` | `despawn::<ScreenRoot>` | `manage_parts_system` |
| EditTop | `spawn_top_editor` | `despawn::<ScreenRoot>` | `text_input_system` → `top_editor_system` |
| EditWeapon | `spawn_weapon_editor` | `despawn::<ScreenRoot>` | `text_input_system` → `weapon_editor_system` |
| EditShaft | `spawn_shaft_editor` | `despawn::<ScreenRoot>` | `text_input_system` → `shaft_editor_system` |
| EditChassis | `spawn_chassis_editor` | `despawn::<ScreenRoot>` | `text_input_system` → `chassis_editor_system` |
| EditScrew | `spawn_screw_editor` | `despawn::<ScreenRoot>` | `text_input_system` → `screw_editor_system` |
| AssembleBuild | `spawn_assemble_build` | `despawn::<ScreenRoot>` | `text_input_system` → `assemble_build_system` |
| PickDesignPart | `spawn_pick_design_part` | `despawn::<ScreenRoot>` | `pick_design_part_system` |
| *（全域）* | — | — | `ui_scroll_system`（Update，無狀態限制） |

---

## DesignState Resource

```rust
pub struct DesignState {
    pub editing_part_id: Option<String>,      // 正在編輯的零件（新增時預先生成）
    pub picking_slot: Option<PartSlot>,        // 在 PickDesignPart 中選擇的槽位（None = 輪盤）
    pub current_build_id: Option<String>,      // 正在編輯的配裝（None = 新配裝）
    pub current_build_top_id: String,
    pub current_build_weapon_id: String,
    pub current_build_shaft_id: String,
    pub current_build_chassis_id: String,
    pub current_build_screw_id: String,
    pub current_build_note: String,
    pub return_to_manage: bool,                // true = 儲存/取消後回到 ManageParts
    pub delete_error: Option<String>,          // 錯誤橫幅文字（顯示後清除）
}
```

### 關鍵行為

- **新增零件流程**：`editing_part_id = Some(gen_custom_id())`，`return_to_manage = false`。儲存 → DesignHub。
- **編輯零件流程**：`editing_part_id = Some(existing_id)`，`return_to_manage = true`。儲存 → ManageParts。
- **刪除錯誤**：當零件被配裝使用時，`DeleteTop`/`DeletePart` 設定此值。下次渲染 ManageParts 時顯示紅色橫幅，然後透過 `.take()` 清除。

---

## 標記組件

| 組件 | 類型 | 使用於 | 用途 |
|------|------|--------|------|
| `ScreenRoot` | Struct | 所有畫面 | 清除錨點，用於 `despawn::<ScreenRoot>` |
| `TextInput` | Struct | 所有編輯器 | 文字輸入欄位（`value`、`focused`、`field_key`） |
| `TextInputDisplay` | Struct | 所有編輯器 | 顯示輸入值的子 Text 實體 |
| `HubButton` | Enum | DesignHub | `NewTop`（顯示為 "New Wheel"）, `NewWeapon`, `NewShaft`, `NewChassis`, `NewScrew`, `ManageParts`, `Back` |
| `ManageButton` | Enum | ManageParts | `EditTop(id)`, `DeleteTop(id)`, `EditPart{slot,id}`, `DeletePart{slot,id}`, `EditBuild(id)`, `DeleteBuild(id)`, `NewBuild`, `Back` |
| `EditorButton` | Enum | 輪盤/軸/底盤/螺絲編輯器 | `Save`, `Cancel`, `SetImage` |
| `WeaponEditorButton` | Enum | 武器編輯器 | `Save`, `Cancel`, `SetImage`, `SetProjectileImage` |
| `KindSelector` | Struct | 武器編輯器 | `current: WeaponKind`，`just_pressed: bool` |
| `KindSelectorLabel` | Struct | 武器編輯器 | 類型按鈕的顯示文字 |
| `MeleeSection` | Struct | 武器編輯器 | 近戰參數欄位的容器 |
| `RangedSection` | Struct | 武器編輯器 | 遠程參數欄位的容器 |
| `AssembleButton` | Enum | AssembleBuild | `ChangeTop`, `ChangeWeapon`, `ChangeShaft`, `ChangeChassis`, `ChangeScrew`, `SaveBuild`, `Back` |
| `StatsPreviewText` | Struct | AssembleBuild | 即時數值預覽顯示 |
| `PickPartButton` | Enum | PickDesignPart | `Select(id)`, `Back` |

---

## 輔助函式

| 函式 | 用途 | 參數 |
|------|------|------|
| `despawn::<T>` | 清除所有帶有組件 T 的實體 | `Query<Entity, With<T>>` |
| `gen_custom_id()` | 從奈秒時間戳產生唯一 ID | → 類似 `"custom_abc123"` 的字串 |
| `slot_dir(slot)` | `PartSlot` → 資產目錄名稱 | `"weapons"`, `"shafts"`, `"chassis"`, `"screws"` |
| `is_builtin(id)` | 檢查 ID 是否為硬編碼預設 | `"default_top"`, `"basic_blade"` 等 |
| `builds_using_part(registry, id)` | 找出所有參照某零件的配裝 | 返回 `Vec<String>` 配裝名稱 |
| `spawn_title(parent, title)` | 36px 青色標題 | — |
| `spawn_button(parent, label, marker)` | 標準按鈕（含標籤 + 標記組件） | 泛型 `C: Component` |
| `spawn_field_row(parent, label, desc, key, default)` | 帶說明的文字輸入欄 | 建立 `TextInput` + `TextInputDisplay` |
| `read_field(inputs, key)` | 以 field_key 讀取文字輸入值 | — |
| `read_f32(inputs, key, default)` | 從文字輸入解析 f32 | — |
| `read_u32(inputs, key, default)` | 從文字輸入解析 u32 | — |
| `hover_system(interaction, bg)` | 標準按鈕 hover 顏色 | BTN → BTN_HOVER → BTN |
| `spawn_image_preview(parent, image, size)` | 圖片節點或深色佔位符 | `Option<Handle<Image>>` |
| `spawn_card_frame(parent, name, stats, image, bg, width, extras)` | 含圖片+名稱+數值+閉包的卡片 | `extras: FnOnce(&mut ChildSpawnerCommands)` |
| `spawn_icon_button(parent, icon, marker)` | 28×28 透明圖示按鈕 | 泛型 `C: Component` |
| `spawn_slot_row(parent, label, name, btn, image)` | 配裝組合槽位列（含圖片） | 用於 AssembleBuild |
| `spawn_pick_card(parent, id, name, stats, image)` | 200px 選擇卡片 | 用於 PickDesignPart |
| `pick_and_copy_image(slot_dir, part_id)` | 開啟檔案選擇器，複製 PNG 到資產 | 使用 `rfd::FileDialog` |
| `kind_display_text(kind)` | `WeaponKind` → 顯示字串 | `"Melee"` 或 `"Ranged"` |
| `next_kind(kind)` | 循環切換武器類型 | Melee → Ranged → Melee |

---

## 狀態轉換（各系統）

### design_hub_system
| 按鈕 | 動作 | 下一個 Phase |
|------|------|-------------|
| NewTop（顯示為 "New Wheel"） | `editing_part_id = gen_custom_id()` | EditTop（標題為 "New Wheel"） |
| NewWeapon | `editing_part_id = gen_custom_id()` | EditWeapon |
| NewShaft | `editing_part_id = gen_custom_id()` | EditShaft |
| NewChassis | `editing_part_id = gen_custom_id()` | EditChassis |
| NewScrew | `editing_part_id = gen_custom_id()` | EditScrew |
| ManageParts | `editing_part_id = None` | ManageParts |
| Back | `editing_part_id = None` | MainMenu |

### manage_parts_system
| 按鈕 | 動作 | 下一個 Phase |
|------|------|-------------|
| EditTop(id) | `editing_part_id = id, return_to_manage = true` | EditTop |
| DeleteTop(id) | 檢查 `builds_using_part` → 刪除或設定錯誤 | ManageParts |
| EditPart{slot,id} | `editing_part_id = id, return_to_manage = true` | Edit(slot) |
| DeletePart{slot,id} | 檢查 `builds_using_part` → 刪除或設定錯誤 | ManageParts |
| EditBuild(id) | `current_build_id = Some(id)` | AssembleBuild |
| DeleteBuild(id) | 從 DB + registry 刪除 | ManageParts |
| NewBuild | 重置所有配裝槽位為預設值 | AssembleBuild |
| Back | — | DesignHub |

### 編輯器系統（陀螺/軸/底盤/螺絲/武器）
| 按鈕 | 動作 | 下一個 Phase |
|------|------|-------------|
| Save | 儲存 JSON 至 SQLite，更新 registry | ManageParts（若 return_to_manage）否則 DesignHub |
| Cancel | — | ManageParts（若 return_to_manage）否則 DesignHub |
| SetImage | `pick_and_copy_image()` | *（同一 Phase，UI 重新整理）* |

### assemble_build_system
| 按鈕 | 動作 | 下一個 Phase |
|------|------|-------------|
| ChangeTop/Weapon/... | 設定 `picking_slot` | PickDesignPart |
| SaveBuild | 儲存至 DB + registry | ManageParts |
| Back | — | ManageParts |

### pick_design_part_system
| 按鈕 | 動作 | 下一個 Phase |
|------|------|-------------|
| Select(id) | 更新 DesignState 中對應的配裝槽位 | AssembleBuild |
| Back | — | AssembleBuild |

---

## 武器編輯器 — 類型切換細節

武器編輯器有特殊的 `KindSelector` 組件：

1. **點擊循環** `Melee → Ranged → Melee`（二元，無 Hybrid）
2. **`just_pressed: bool`** 防止按住時每幀都切換
3. **`MeleeSection` / `RangedSection`** 容器透過 `Display::None` / `Display::Flex` 切換
   - 曾嘗試 `Visibility::Hidden` 但會保留佈局空間 → 使用 `Display::None`
4. **儲存時**：讀取 `KindSelector.current` 決定建立哪種規格
   - `is_melee` / `is_ranged` 互斥
   - 只填充激活的規格；另一個為 `None`

---

## 零件刪除 — 參照完整性

1. `builds_using_part(registry, part_id)` 檢查所有 `BuildRef` 欄位
2. 若非空 → `state.delete_error = Some(message)`，不刪除
3. 下次渲染 ManageParts → 捲動區域頂部顯示紅色橫幅
4. `state.delete_error.take()` 清除（單次顯示）

---

## UI 捲動系統

- `ui_scroll_system` 在 `Update` 執行（無狀態限制）
- 讀取 `MessageReader<MouseWheel>`，使用 Bevy picking 的 `HoverMap`
- 更新有 `ScrollPosition` 的懸停實體的 `ScrollPosition.y`
- 所有 `Overflow::scroll_y()` 容器**也必須**有 `ScrollPosition::default()`
- `SCROLL_LINE_HEIGHT = 21.0` px（每個滾輪單位）

---

## ManageParts 畫面佈局

```
┌────────────────────────────────┐
│  固定頂部列：「我的零件與配裝」   │  （不可捲動）
│  標題                          │
├────────────────────────────────┤
│  可捲動中間區域：               │  Overflow::scroll_y()
│    [錯誤橫幅，若有]             │  + ScrollPosition::default()
│    陀螺區段（卡片格）           │
│    武器區段                     │
│    軸區段                       │
│    底盤區段                     │
│    螺絲區段                     │
│    配裝區段                     │
│    [底部留白]                   │
├────────────────────────────────┤
│  固定底部列：                   │  （不可捲動）
│  [新增配裝] [返回]              │
└────────────────────────────────┘
```

每個區段：標題文字 → 卡片格（含編輯/刪除圖示按鈕）。
內建零件顯示「(內建)」而非按鈕。
