# UI 模式與 Bevy 0.18 慣例

> 所有 Plugin 中建立 UI 的方式。修改任何畫面前請先閱讀本文件。

---

## 畫面生命週期模式

每個畫面都遵循相同模式：

```rust
// Plugin::build
app.add_systems(OnEnter(GamePhase::X), spawn_x);
app.add_systems(OnExit(GamePhase::X),  despawn::<MarkerComponent>);
app.add_systems(Update, x_system.run_if(in_state(GamePhase::X)));
```

- **OnEnter**：spawn 函式建立整個 UI 樹狀結構
- **OnExit**：`despawn::<T>` 移除根實體（Bevy 0.18 預設遞歸）
- **Update**：系統處理按鈕的 `Interaction`，讀取 `TextInput`，轉換狀態
- **根標記**：`ScreenRoot`（設計插件）、`MainMenuRoot` / `SelectionRoot` / `PickerRoot`（選單插件）

---

## 可捲動容器配方

**滑鼠滾輪捲動需要以下三個條件缺一不可：**

```rust
parent.spawn((
    Node {
        overflow: Overflow::scroll_y(),
        // ... 其他佈局
        ..default()
    },
    ScrollPosition::default(),  // 必要：缺少此項，捲動位移永遠為 0
));
```

加上設計插件中的全域 `ui_scroll_system` 處理 `MouseWheel` → `ScrollPosition` 更新（使用 `HoverMap`）。

### 捲動容器約束（flex 子元素）

當捲動容器是 flex 子元素且不應超出父元素時：

```rust
Node {
    flex_grow: 1.0,
    flex_shrink: 1.0,
    flex_basis: Val::Px(0.0),
    min_height: Val::Px(0.0),
    overflow: Overflow::scroll_y(),
    ..default()
}
```

缺少 `flex_shrink + flex_basis + min_height` 時，內容會撐破容器超出視口。

### 固定頁首 / 捲動 / 固定頁尾佈局

```rust
outer.spawn(Node {
    width: Val::Percent(100.0),
    height: Val::Percent(100.0),
    flex_direction: FlexDirection::Column,
    ..default()
}).with_children(|outer| {
    // 固定頁首
    outer.spawn(Node { padding: ..., ..default() }).with_children(|bar| { ... });

    // 可捲動中間（flex_grow: 1.0 + 捲動約束）
    outer.spawn((
        Node { flex_grow: 1.0, flex_shrink: 1.0, flex_basis: Val::Px(0.0),
               min_height: Val::Px(0.0), overflow: Overflow::scroll_y(), ..default() },
        ScrollPosition::default(),
    )).with_children(|root| { ... });

    // 固定頁尾
    outer.spawn(Node { padding: ..., ..default() }).with_children(|row| { ... });
});
```

使用於：ManageParts 畫面。

---

## 按鈕模式

```rust
fn spawn_button<C: Component>(parent, label: &str, marker: C)
```

建立：`(marker, Button, Node { ... }, BackgroundColor(COLOR_BTN))` → 子 `Text`。

hover 處理：每個系統匹配 `(Interaction, &ButtonMarker, &mut BackgroundColor)`：
- 標準按鈕：`hover_system(interaction, &mut bg)` → BTN / BTN_HOVER
- 圖示按鈕：自訂 hover → 透明 / srgba(0.4, 0.4, 0.5, 0.3)

---

## 圖示按鈕模式

```rust
fn spawn_icon_button<C: Component>(parent, icon: Handle<Image>, marker: C)
```

建立：28×28 透明按鈕，含 `ImageNode` 子元素（24×24）。

用於 ManageParts 卡片上的編輯/刪除按鈕。

---

## 卡片模式

```rust
fn spawn_card_frame(parent, name, stats_line, image, bg_color, width, spawn_extras)
```

建立：`Node { width, 欄方向佈局, padding, border_radius }` → 圖片預覽 → 名稱文字 → 數值文字 → `spawn_extras(card)` 閉包。

`spawn_extras` 閉包添加編輯/刪除按鈕或「(內建)」標籤。

---

## 文字輸入模式

```rust
fn spawn_field_row(parent, label, description, field_key, default_value)
```

建立：含標籤 + 說明 + `TextInput` 組件 + `TextInputDisplay` 子元素的列。

- `TextInput { value, focused, field_key }` — 資料
- `TextInputDisplay` — 可見的 `Text` 實體
- `text_input_system` 處理焦點、鍵盤輸入、退格
- 讀取值：`read_field(inputs, "key")`、`read_f32(inputs, "key", default)`

---

## Bevy 0.18 API 注意事項

### 必知事項

| 問題 | 正確做法 |
|------|---------|
| `with_children` 閉包型別 | `ChildSpawnerCommands`（非 `ChildBuilder`） |
| 清除實體 | `despawn()` 是遞歸的（沒有 `despawn_recursive()`） |
| `BorderRadius` | 是 `Node` 上的欄位，非獨立 Component |
| 事件 | `MessageWriter<T>` / `MessageReader<T>`（非 `Events<T>`） |
| 鍵盤事件 | `MessageReader<KeyboardInput>`（非 `EventReader`） |
| 滑鼠滾輪 | `MessageReader<MouseWheel>` |
| Query 衝突 B0001 | 使用 `Without<T>` 證明不相交；`Changed<T>` 無效 |
| Bundle tuple 限制 | 最多約 15 個元素 — 超過時需巢狀 tuple |
| Hover 偵測 | `HoverMap`（來自 `bevy::picking::hover`，0.15 起內建） |

### Display vs Visibility

| 屬性 | 對佈局的影響 |
|------|------------|
| `Display::None` | 完全從佈局移除（不保留空間） |
| `Display::Flex` | 正常 flex 佈局 |
| `Visibility::Hidden` | **仍保留佈局空間**（隱藏但存在） |

**切換應折疊的區段時使用 `Display::None/Flex`**（例如武器編輯器的近戰/遠程區段）。

### 捲動容器上的 JustifyContent

`JustifyContent::Center` 在溢出時**會裁剪頂部內容**。改用 `JustifyContent::FlexStart` 並加頂部 padding。

---

## 色彩調色盤

### 設計插件（深色主題）
```
BG:          srgba(0.08, 0.08, 0.12)   // 近乎黑色
BTN:         srgba(0.18, 0.20, 0.28)   // 深藍灰
BTN_HOVER:   srgba(0.28, 0.32, 0.42)   // 較淺的藍灰
ACCENT:      srgba(0.2, 0.7, 1.0)      // 青色（標題）
CARD:        srgba(0.12, 0.14, 0.20)   // 卡片背景
INPUT_BG:    srgba(0.10, 0.10, 0.16)
INPUT_FOCUS: srgba(0.15, 0.15, 0.25)
TEXT:        WHITE
TEXT_DIM:    srgba(0.5, 0.5, 0.5)
```

### 選單插件（略有不同）
```
BG:          srgba(0.075, 0.075, 0.09)
BTN:         srgba(0.18, 0.20, 0.27)
BTN_HOVER:   srgba(0.28, 0.32, 0.42)
SELECTED:    srgba(0.14, 0.45, 0.75)
ACCENT:      srgba(0.2, 0.7, 1.0)      // 同樣的青色
```
