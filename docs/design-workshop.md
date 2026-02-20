# Design Workshop — `src/plugins/design_plugin.rs`

> Everything about the Design Wheel / Workshop feature lives in one file.
> This doc is the full reference for that file's structure.

---

## State Machine

```
MainMenu
  └─ DesignHub
       ├─ EditTop        (create / edit wheel)
       ├─ EditWeapon     (create / edit weapon)
       ├─ EditShaft      (create / edit shaft)
       ├─ EditChassis    (create / edit chassis)
       ├─ EditScrew      (create / edit screw)
       └─ ManageParts    (list all parts & builds)
            ├─ Edit*     (edit existing part → return_to_manage=true)
            ├─ AssembleBuild
            │    └─ PickDesignPart  (pick part for a slot)
            └─ NewBuild → AssembleBuild
```

### Phase → System Registration (Plugin::build)

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
| *(global)* | — | — | `ui_scroll_system` (Update, no state gate) |

---

## DesignState Resource

```rust
pub struct DesignState {
    pub editing_part_id: Option<String>,      // Part being edited (pre-generated for new)
    pub picking_slot: Option<PartSlot>,        // Slot being picked in PickDesignPart (None = wheel)
    pub current_build_id: Option<String>,      // Build being edited (None = new build)
    pub current_build_top_id: String,
    pub current_build_weapon_id: String,
    pub current_build_shaft_id: String,
    pub current_build_chassis_id: String,
    pub current_build_screw_id: String,
    pub current_build_note: String,
    pub return_to_manage: bool,                // true = return to ManageParts after save/cancel
    pub delete_error: Option<String>,          // Error banner text (cleared after display)
}
```

### Key Behaviors

- **New part flow**: `editing_part_id = Some(gen_custom_id())`, `return_to_manage = false`. Save → DesignHub.
- **Edit part flow**: `editing_part_id = Some(existing_id)`, `return_to_manage = true`. Save → ManageParts.
- **Delete error**: Set by `DeleteTop`/`DeletePart` when part is used by builds. Displayed as red banner on next ManageParts render, then cleared via `.take()`.

---

## Marker Components

| Component | Type | Used In | Purpose |
|-----------|------|---------|---------|
| `ScreenRoot` | Struct | All screens | Despawn anchor for `despawn::<ScreenRoot>` |
| `TextInput` | Struct | All editors | Text input field (`value`, `focused`, `field_key`) |
| `TextInputDisplay` | Struct | All editors | Child Text entity showing input value |
| `HubButton` | Enum | DesignHub | `NewTop` (label: "New Wheel"), `NewWeapon`, `NewShaft`, `NewChassis`, `NewScrew`, `ManageParts`, `Back` |
| `ManageButton` | Enum | ManageParts | `EditTop(id)`, `DeleteTop(id)`, `EditPart{slot,id}`, `DeletePart{slot,id}`, `EditBuild(id)`, `DeleteBuild(id)`, `NewBuild`, `Back` |
| `EditorButton` | Enum | Wheel/Shaft/Chassis/Screw editors | `Save`, `Cancel`, `SetImage` |
| `WeaponEditorButton` | Enum | Weapon editor | `Save`, `Cancel`, `SetImage`, `SetProjectileImage` |
| `KindSelector` | Struct | Weapon editor | `current: WeaponKind`, `just_pressed: bool` |
| `KindSelectorLabel` | Struct | Weapon editor | Display text for kind button |
| `MeleeSection` | Struct | Weapon editor | Container for melee param fields |
| `RangedSection` | Struct | Weapon editor | Container for ranged param fields |
| `AssembleButton` | Enum | AssembleBuild | `ChangeTop`, `ChangeWeapon`, `ChangeShaft`, `ChangeChassis`, `ChangeScrew`, `SaveBuild`, `Back` |
| `StatsPreviewText` | Struct | AssembleBuild | Live stats preview display |
| `PickPartButton` | Enum | PickDesignPart | `Select(id)`, `Back` |

---

## Helper Functions

| Function | Purpose | Params |
|----------|---------|--------|
| `despawn::<T>` | Despawn all entities with component T | `Query<Entity, With<T>>` |
| `gen_custom_id()` | Unique ID from nanosecond timestamp | → `String` like `"custom_abc123"` |
| `slot_dir(slot)` | `PartSlot` → asset directory name | `"weapons"`, `"shafts"`, `"chassis"`, `"screws"` |
| `is_builtin(id)` | Check if ID is a hardcoded default | `"default_top"`, `"basic_blade"`, `"basic_blaster"`, `"standard_shaft"`, `"standard_chassis"`, `"standard_screw"`, `"default_blade"`, `"default_blaster"` |
| `builds_using_part(registry, id)` | Find all builds referencing a part | Returns `Vec<String>` of build names |
| `spawn_title(parent, title)` | 36px cyan accent title | — |
| `spawn_button(parent, label, marker)` | Standard button with label + marker component | Generic `C: Component` |
| `spawn_field_row(parent, label, desc, key, default)` | Labeled text input with description | Creates `TextInput` + `TextInputDisplay` |
| `read_field(inputs, key)` | Read text input value by field_key | — |
| `read_f32(inputs, key, default)` | Parse f32 from text input | — |
| `read_u32(inputs, key, default)` | Parse u32 from text input | — |
| `hover_system(interaction, bg)` | Standard button hover colors | BTN → BTN_HOVER → BTN |
| `spawn_image_preview(parent, image, size)` | Image node or dark placeholder | `Option<Handle<Image>>` |
| `spawn_card_frame(parent, name, stats, image, bg, width, extras)` | Card with image+name+stats+closure | `extras: FnOnce(&mut ChildSpawnerCommands)` |
| `spawn_icon_button(parent, icon, marker)` | 28x28 transparent icon button | Generic `C: Component` |
| `spawn_slot_row(parent, label, name, btn, image)` | Build assembly slot row with image | Used in AssembleBuild |
| `spawn_pick_card(parent, id, name, stats, image)` | 200px selection card for PickDesignPart | — |
| `pick_and_copy_image(slot_dir, part_id)` | Opens file picker, copies PNG to assets | Uses `rfd::FileDialog` |
| `kind_display_text(kind)` | `WeaponKind` → display string | `"Melee"` or `"Ranged"` |
| `next_kind(kind)` | Cycle weapon kind | Melee → Ranged → Melee |

### Section Spawners (ManageParts grid sections)

| Function | Section | Card Function |
|----------|---------|---------------|
| `spawn_section_with_tops` | "Wheels" | `spawn_top_card` |
| `spawn_section_with_parts` | "Weapons" | `spawn_part_card` |
| `spawn_section_with_shafts` | "Shafts" | `spawn_part_card` |
| `spawn_section_with_chassis` | "Chassis" | `spawn_part_card` |
| `spawn_section_with_screws` | "Screws" | `spawn_part_card` |
| `spawn_section_with_builds` | "Builds" | `spawn_card_frame` (inline) |

---

## State Transitions (from each system)

### design_hub_system
| Button | Action | Next Phase |
|--------|--------|------------|
| NewTop (shown as "New Wheel") | `editing_part_id = gen_custom_id()` | EditTop (titled "New Wheel") |
| NewWeapon | `editing_part_id = gen_custom_id()` | EditWeapon |
| NewShaft | `editing_part_id = gen_custom_id()` | EditShaft |
| NewChassis | `editing_part_id = gen_custom_id()` | EditChassis |
| NewScrew | `editing_part_id = gen_custom_id()` | EditScrew |
| ManageParts | `editing_part_id = None` | ManageParts |
| Back | `editing_part_id = None` | MainMenu |

### manage_parts_system
| Button | Action | Next Phase |
|--------|--------|------------|
| EditTop(id) | `editing_part_id = id, return_to_manage = true` | EditTop |
| DeleteTop(id) | Check `builds_using_part` → delete or set error | ManageParts |
| EditPart{slot,id} | `editing_part_id = id, return_to_manage = true` | Edit(slot) |
| DeletePart{slot,id} | Check `builds_using_part` → delete or set error | ManageParts |
| EditBuild(id) | `current_build_id = Some(id)` | AssembleBuild |
| DeleteBuild(id) | Delete from DB + registry | ManageParts |
| NewBuild | Reset all build slots to defaults | AssembleBuild |
| Back | — | DesignHub |

### Editor systems (top/shaft/chassis/screw/weapon)
| Button | Action | Next Phase |
|--------|--------|------------|
| Save | Save JSON to SQLite, update registry | ManageParts (if return_to_manage) else DesignHub |
| Cancel | — | ManageParts (if return_to_manage) else DesignHub |
| SetImage | `pick_and_copy_image()` | *(same phase, UI refresh)* |

### assemble_build_system
| Button | Action | Next Phase |
|--------|--------|------------|
| ChangeTop/Weapon/... | Set `picking_slot` | PickDesignPart |
| SaveBuild | Save to DB + registry | ManageParts |
| Back | — | ManageParts |

### pick_design_part_system
| Button | Action | Next Phase |
|--------|--------|------------|
| Select(id) | Update corresponding build slot in DesignState | AssembleBuild |
| Back | — | AssembleBuild |

---

## Color Constants

```rust
COLOR_BG:            srgba(0.08, 0.08, 0.12, 1.0)   // Dark background
COLOR_BTN:           srgba(0.18, 0.20, 0.28, 1.0)   // Button default
COLOR_BTN_HOVER:     srgba(0.28, 0.32, 0.42, 1.0)   // Button hovered
COLOR_BTN_PRESS:     srgba(0.12, 0.14, 0.20, 1.0)   // Button pressed (unused)
COLOR_TEXT:          WHITE
COLOR_TEXT_DIM:      srgba(0.5, 0.5, 0.5, 1.0)
COLOR_ACCENT:        srgba(0.2, 0.7, 1.0, 1.0)       // Cyan, titles
COLOR_CARD:          srgba(0.12, 0.14, 0.20, 1.0)     // Card background
COLOR_CARD_SELECTED: srgba(0.15, 0.35, 0.60, 1.0)     // Selected card
COLOR_INPUT_BG:      srgba(0.10, 0.10, 0.16, 1.0)
COLOR_INPUT_FOCUS:   srgba(0.15, 0.15, 0.25, 1.0)
COLOR_DANGER:        srgba(0.8, 0.2, 0.2, 1.0)        // Red (unused const, inline used)
COLOR_SUCCESS:       srgba(0.2, 0.7, 0.3, 1.0)        // Green (unused)
```

---

## Weapon Editor — Kind Toggle Details

The weapon editor has a special `KindSelector` component:

1. **Click cycles** `Melee → Ranged → Melee` (binary, no Hybrid)
2. **`just_pressed: bool`** prevents cycling every frame while held
3. **`MeleeSection` / `RangedSection`** containers toggle via `Display::None` / `Display::Flex`
   - `Visibility::Hidden` was tried but reserves layout space → use `Display::None`
4. **On save**: reads `KindSelector.current` to decide which spec to build
   - `is_melee` / `is_ranged` are mutually exclusive booleans
   - Only the active spec is populated; the other is `None`

---

## Part Deletion — Referential Integrity

1. `builds_using_part(registry, part_id)` checks all `BuildRef` fields
2. If non-empty → `state.delete_error = Some(message)`, no deletion
3. On next ManageParts render → red banner at top of scroll area
4. `state.delete_error.take()` clears it (one-shot display)

---

## UI Scroll System

- `ui_scroll_system` runs on `Update` (not state-gated)
- Reads `MessageReader<MouseWheel>`, uses `HoverMap` from Bevy picking
- Updates `ScrollPosition.y` on hovered entities that have `ScrollPosition`
- All `Overflow::scroll_y()` containers **must** also have `ScrollPosition::default()`
- `SCROLL_LINE_HEIGHT = 21.0` px per mouse wheel line unit

---

## ManageParts Screen Layout

```
┌────────────────────────────────┐
│  Fixed top bar: "My Parts &    │  (not scrollable)
│  Builds" title                 │
├────────────────────────────────┤
│  Scrollable middle area:       │  Overflow::scroll_y()
│    [Error banner if any]       │  + ScrollPosition::default()
│    Tops section (card grid)    │
│    Weapons section             │
│    Shafts section              │
│    Chassis section             │
│    Screws section              │
│    Builds section              │
│    [bottom padding]            │
├────────────────────────────────┤
│  Fixed bottom bar:             │  (not scrollable)
│  [New Build] [Back]            │
└────────────────────────────────┘
```

Each section: header text → `Row+Wrap` grid of cards with edit/delete icon buttons.
Built-in parts show "(built-in)" instead of buttons.
