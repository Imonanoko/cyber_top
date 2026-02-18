# UI Patterns & Bevy 0.18 Conventions

> How UI is built across all plugins. Read this before modifying any screen.

---

## Screen Lifecycle Pattern

Every screen follows the same pattern:

```rust
// Plugin::build
app.add_systems(OnEnter(GamePhase::X), spawn_x);
app.add_systems(OnExit(GamePhase::X),  despawn::<MarkerComponent>);
app.add_systems(Update, x_system.run_if(in_state(GamePhase::X)));
```

- **OnEnter**: spawn function builds entire UI tree
- **OnExit**: `despawn::<T>` removes root entity (recursive by default in Bevy 0.18)
- **Update**: system handles `Interaction` on buttons, reads `TextInput`, transitions state
- **Root marker**: `ScreenRoot` (design plugin), `MainMenuRoot` / `SelectionRoot` / `PickerRoot` (menu plugin)

---

## Scrollable Container Recipe

**All three are required** for mouse wheel scrolling to work:

```rust
parent.spawn((
    Node {
        overflow: Overflow::scroll_y(),
        // ... other layout
        ..default()
    },
    ScrollPosition::default(),  // REQUIRED: without this, scroll offset stays 0
));
```

Plus the global `ui_scroll_system` in DesignPlugin handles `MouseWheel` → `ScrollPosition` updates using `HoverMap`.

### Scroll Container Constraints (for flex children)

When a scroll container is a flex child that should not exceed its parent:

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

Without `flex_shrink + flex_basis + min_height`, content pushes container beyond viewport.

### Fixed Header / Scroll / Fixed Footer Layout

```rust
outer.spawn(Node {
    width: Val::Percent(100.0),
    height: Val::Percent(100.0),
    flex_direction: FlexDirection::Column,
    ..default()
}).with_children(|outer| {
    // Fixed header
    outer.spawn(Node { padding: ..., ..default() }).with_children(|bar| { ... });

    // Scrollable middle (flex_grow: 1.0 + scroll constraints)
    outer.spawn((
        Node { flex_grow: 1.0, flex_shrink: 1.0, flex_basis: Val::Px(0.0),
               min_height: Val::Px(0.0), overflow: Overflow::scroll_y(), ..default() },
        ScrollPosition::default(),
    )).with_children(|root| { ... });

    // Fixed footer
    outer.spawn(Node { padding: ..., ..default() }).with_children(|row| { ... });
});
```

Used in: ManageParts screen.

---

## Button Pattern

```rust
fn spawn_button<C: Component>(parent, label: &str, marker: C)
```

Creates: `(marker, Button, Node { ... }, BackgroundColor(COLOR_BTN))` → child `Text`.

Hover handling: each system matches on `(Interaction, &ButtonMarker, &mut BackgroundColor)`:
- Standard buttons: `hover_system(interaction, &mut bg)` → BTN / BTN_HOVER
- Icon buttons: custom hover → transparent / srgba(0.4, 0.4, 0.5, 0.3)

---

## Icon Button Pattern

```rust
fn spawn_icon_button<C: Component>(parent, icon: Handle<Image>, marker: C)
```

Creates: 28x28 transparent button with `ImageNode` child (24x24).

Used for edit/delete buttons on cards in ManageParts.

---

## Card Pattern

```rust
fn spawn_card_frame(parent, name, stats_line, image, bg_color, width, spawn_extras)
```

Creates: `Node { width, column layout, padding, border_radius }` → image preview → name text → stats text → `spawn_extras(card)` closure.

The `spawn_extras` closure adds edit/delete buttons or "(built-in)" label.

---

## Text Input Pattern

```rust
fn spawn_field_row(parent, label, description, field_key, default_value)
```

Creates: row with label + description + `TextInput` component + `TextInputDisplay` child.

- `TextInput { value, focused, field_key }` — the data
- `TextInputDisplay` — the visible `Text` entity
- `text_input_system` handles focus, keyboard input, backspace
- Read values: `read_field(inputs, "key")`, `read_f32(inputs, "key", default)`

---

## Bevy 0.18 API Gotchas

### Must-know

| Gotcha | Correct Way |
|--------|-------------|
| `with_children` closure type | `ChildSpawnerCommands` (not `ChildBuilder`) |
| Despawn | `despawn()` is recursive (no `despawn_recursive()`) |
| `BorderRadius` | Field on `Node`, not a separate Component |
| Events | `MessageWriter<T>` / `MessageReader<T>` (not `Events<T>`) |
| Keyboard events | `MessageReader<KeyboardInput>` (not `EventReader`) |
| Mouse wheel | `MessageReader<MouseWheel>` |
| Query conflict B0001 | Use `Without<T>` to prove disjointness; `Changed<T>` does NOT help |
| Bundle tuple limit | ~15 elements max — nest inner tuples if needed |
| Hover detection | `HoverMap` from `bevy::picking::hover` (built-in since 0.15) |

### Display vs Visibility

| Property | Effect on Layout |
|----------|-----------------|
| `Display::None` | Removed from layout entirely (no space reserved) |
| `Display::Flex` | Normal flex layout |
| `Visibility::Hidden` | **Still reserves layout space** (hidden but present) |

**Use `Display::None/Flex`** when toggling sections that should collapse (e.g. weapon editor Melee/Ranged sections).

### JustifyContent on Scrollable Containers

`JustifyContent::Center` on a scroll container **clips top content** when overflowing. Use `JustifyContent::FlexStart` with top padding instead.

---

## Color Palettes

### Design Plugin (dark theme)
```
BG:          srgba(0.08, 0.08, 0.12)   // Near-black
BTN:         srgba(0.18, 0.20, 0.28)   // Dark blue-gray
BTN_HOVER:   srgba(0.28, 0.32, 0.42)   // Lighter blue-gray
ACCENT:      srgba(0.2, 0.7, 1.0)      // Cyan (titles)
CARD:        srgba(0.12, 0.14, 0.20)   // Card background
INPUT_BG:    srgba(0.10, 0.10, 0.16)
INPUT_FOCUS: srgba(0.15, 0.15, 0.25)
TEXT:         WHITE
TEXT_DIM:     srgba(0.5, 0.5, 0.5)
```

### Menu Plugin (similar but slightly different)
```
BG:          srgba(0.075, 0.075, 0.09)
BTN:         srgba(0.18, 0.20, 0.27)
BTN_HOVER:   srgba(0.28, 0.32, 0.42)
SELECTED:    srgba(0.14, 0.45, 0.75)
ACCENT:      srgba(0.2, 0.7, 1.0)      // Same cyan
```
