use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use std::time::SystemTime;

use crate::game::components::GamePhase;
use crate::game::map::{is_valid_placement, MapItem, MapPlacement, MapSpec, GRID_CELL_SIZE};
use crate::game::parts::registry::PartRegistry;
use crate::plugins::storage_plugin::TokioRuntime;
use crate::storage::sqlite_repo::SqliteRepo;

// ── Colors (same palette as design_plugin) ─────────────────────────

const COLOR_BG: Color = Color::srgba(0.08, 0.08, 0.12, 1.0);
const COLOR_BTN: Color = Color::srgba(0.18, 0.20, 0.28, 1.0);
const COLOR_BTN_HOVER: Color = Color::srgba(0.28, 0.32, 0.42, 1.0);
const COLOR_BTN_PRESS: Color = Color::srgba(0.12, 0.14, 0.20, 1.0);
const COLOR_TEXT: Color = Color::WHITE;
const COLOR_TEXT_DIM: Color = Color::srgba(0.5, 0.5, 0.5, 1.0);
const COLOR_ACCENT: Color = Color::srgba(0.2, 0.7, 1.0, 1.0);
const COLOR_CARD: Color = Color::srgba(0.12, 0.14, 0.20, 1.0);
const COLOR_INPUT_BG: Color = Color::srgba(0.10, 0.10, 0.16, 1.0);
const COLOR_INPUT_FOCUS: Color = Color::srgba(0.15, 0.15, 0.25, 1.0);
const COLOR_DANGER: Color = Color::srgba(0.8, 0.2, 0.2, 1.0);
const COLOR_TOOL_SELECTED: Color = Color::srgba(0.15, 0.35, 0.60, 1.0);
const COLOR_GRID_EMPTY: Color = Color::srgba(0.12, 0.12, 0.18, 1.0);
const COLOR_GRID_INVALID: Color = Color::srgba(0.06, 0.06, 0.08, 1.0);
const COLOR_GRID_HOVER: Color = Color::srgba(0.25, 0.25, 0.35, 1.0);

// ── Plugin ──────────────────────────────────────────────────────────

pub struct MapDesignPlugin;

impl Plugin for MapDesignPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapDesignState>();

        // DesignMapHub
        app.add_systems(OnEnter(GamePhase::DesignMapHub), spawn_map_hub);
        app.add_systems(OnExit(GamePhase::DesignMapHub), despawn::<MapScreenRoot>);
        app.add_systems(
            Update,
            map_hub_system.run_if(in_state(GamePhase::DesignMapHub)),
        );

        // EditMap
        app.add_systems(OnEnter(GamePhase::EditMap), spawn_map_editor);
        app.add_systems(OnExit(GamePhase::EditMap), despawn::<MapScreenRoot>);
        app.add_systems(
            Update,
            (map_text_input_system, map_editor_system)
                .chain()
                .run_if(in_state(GamePhase::EditMap)),
        );
    }
}

// ── Markers ─────────────────────────────────────────────────────────

#[derive(Component)]
struct MapScreenRoot;

#[derive(Component)]
enum MapHubButton {
    NewMap,
    EditMap(String),
    DeleteMap(String),
    Back,
}

#[derive(Component)]
enum MapEditorButton {
    Save,
    Cancel,
    SelectTool(ToolSelection),
}

#[derive(Component)]
struct GridCell {
    grid_x: i32,
    grid_y: i32,
}

#[derive(Component)]
struct GridContainer;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct MapTextInput {
    value: String,
    focused: bool,
    field_key: String,
}

#[derive(Component)]
struct MapTextInputDisplay;

// ── State ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSelection {
    Obstacle,
    GravityDevice,
    SpeedBoost,
    DamageBoost,
    Erase,
}

impl ToolSelection {
    fn display_name(self) -> &'static str {
        match self {
            Self::Obstacle => "Obstacle",
            Self::GravityDevice => "Gravity",
            Self::SpeedBoost => "Speed",
            Self::DamageBoost => "Damage",
            Self::Erase => "Erase",
        }
    }

    fn to_map_item(self) -> Option<MapItem> {
        match self {
            Self::Obstacle => Some(MapItem::Obstacle),
            Self::GravityDevice => Some(MapItem::GravityDevice),
            Self::SpeedBoost => Some(MapItem::SpeedBoost),
            Self::DamageBoost => Some(MapItem::DamageBoost),
            Self::Erase => None,
        }
    }
}

#[derive(Resource)]
pub struct MapDesignState {
    pub editing_map_id: Option<String>,
    pub current_spec: MapSpec,
    pub selected_tool: ToolSelection,
    pub delete_error: Option<String>,
}

impl Default for MapDesignState {
    fn default() -> Self {
        Self {
            editing_map_id: None,
            current_spec: MapSpec::default_arena(),
            selected_tool: ToolSelection::Obstacle,
            delete_error: None,
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn despawn<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn gen_custom_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("map_{:08x}", nanos)
}

fn is_builtin_map(id: &str) -> bool {
    id == "default_arena"
}

fn spawn_button<C: Component>(parent: &mut ChildSpawnerCommands, label: &str, marker: C) {
    parent
        .spawn((
            marker,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(COLOR_BTN),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

fn spawn_icon_button<C: Component>(
    parent: &mut ChildSpawnerCommands,
    icon: Handle<Image>,
    marker: C,
) {
    parent
        .spawn((
            marker,
            Button,
            Node {
                width: Val::Px(28.0),
                height: Val::Px(28.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|btn| {
            btn.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(24.0),
                    height: Val::Px(24.0),
                    ..default()
                },
            ));
        });
}

// ═══════════════════════════════════════════════════════════════════════
// MAP HUB (DesignMapHub)
// ═══════════════════════════════════════════════════════════════════════

fn spawn_map_hub(
    mut commands: Commands,
    registry: Res<PartRegistry>,
    asset_server: Res<AssetServer>,
    mut state: ResMut<MapDesignState>,
) {
    let edit_icon: Handle<Image> = asset_server.load("ui/edit.png");
    let delete_icon: Handle<Image> = asset_server.load("ui/delete.png");

    // Show and clear delete error
    let error_msg = state.delete_error.take();

    commands
        .spawn((
            MapScreenRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(COLOR_BG),
        ))
        .with_children(|outer| {
            // ── Fixed header ──
            outer
                .spawn(Node {
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                })
                .with_children(|bar| {
                    bar.spawn((
                        Text::new("My Maps"),
                        TextFont {
                            font_size: 36.0,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT),
                    ));
                });

            // ── Scrollable middle ──
            outer
                .spawn((
                    Node {
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        flex_basis: Val::Px(0.0),
                        min_height: Val::Px(0.0),
                        overflow: Overflow::scroll_y(),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::horizontal(Val::Px(24.0)),
                        row_gap: Val::Px(16.0),
                        ..default()
                    },
                    ScrollPosition::default(),
                ))
                .with_children(|root| {
                    // Error banner
                    if let Some(err) = error_msg {
                        root.spawn((
                            Node {
                                padding: UiRect::all(Val::Px(12.0)),
                                margin: UiRect::bottom(Val::Px(8.0)),
                                border_radius: BorderRadius::all(Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(COLOR_DANGER),
                        ))
                        .with_children(|banner| {
                            banner.spawn((
                                Text::new(err),
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT),
                            ));
                        });
                    }

                    // Map cards grid
                    root.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(16.0),
                        row_gap: Val::Px(16.0),
                        ..default()
                    })
                    .with_children(|grid| {
                        let mut maps: Vec<_> = registry.maps.values().collect();
                        maps.sort_by(|a, b| a.name.cmp(&b.name));

                        for map in maps {
                            let placements_text = format!(
                                "R={:.0}, {} items",
                                map.arena_radius,
                                map.placements.len()
                            );

                            grid.spawn((
                                Node {
                                    width: Val::Px(200.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(16.0)),
                                    row_gap: Val::Px(8.0),
                                    border_radius: BorderRadius::all(Val::Px(10.0)),
                                    ..default()
                                },
                                BackgroundColor(COLOR_CARD),
                            ))
                            .with_children(|card| {
                                // Arena preview circle
                                card.spawn((
                                    Node {
                                        width: Val::Px(80.0),
                                        height: Val::Px(80.0),
                                        border_radius: BorderRadius::all(Val::Px(40.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 1.0)),
                                ));

                                // Name
                                card.spawn((
                                    Text::new(&map.name),
                                    TextFont {
                                        font_size: 18.0,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT),
                                ));

                                // Stats
                                card.spawn((
                                    Text::new(&placements_text),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_DIM),
                                ));

                                // Buttons row
                                if is_builtin_map(&map.id) {
                                    card.spawn((
                                        Text::new("(built-in)"),
                                        TextFont {
                                            font_size: 13.0,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT_DIM),
                                    ));
                                } else {
                                    card.spawn(Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(8.0),
                                        ..default()
                                    })
                                    .with_children(|row| {
                                        spawn_icon_button(
                                            row,
                                            edit_icon.clone(),
                                            MapHubButton::EditMap(map.id.clone()),
                                        );
                                        spawn_icon_button(
                                            row,
                                            delete_icon.clone(),
                                            MapHubButton::DeleteMap(map.id.clone()),
                                        );
                                    });
                                }
                            });
                        }
                    });

                    // Bottom padding
                    root.spawn(Node {
                        height: Val::Px(40.0),
                        ..default()
                    });
                });

            // ── Fixed footer ──
            outer
                .spawn(Node {
                    padding: UiRect::all(Val::Px(16.0)),
                    column_gap: Val::Px(12.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|row| {
                    spawn_button(row, "New Map", MapHubButton::NewMap);
                    spawn_button(row, "Back", MapHubButton::Back);
                });
        });
}

fn map_hub_system(
    mut q: Query<(&Interaction, &MapHubButton, &mut BackgroundColor), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut state: ResMut<MapDesignState>,
    mut registry: ResMut<PartRegistry>,
    repo: Option<Res<SqliteRepo>>,
    rt: Option<Res<TokioRuntime>>,
) {
    for (interaction, button, mut bg) in &mut q {
        match button {
            MapHubButton::NewMap => match *interaction {
                Interaction::Pressed => {
                    let id = gen_custom_id();
                    state.editing_map_id = Some(id.clone());
                    state.current_spec = MapSpec {
                        id,
                        name: "New Map".into(),
                        arena_radius: 12.0,
                        placements: vec![],
                    };
                    next_state.set(GamePhase::EditMap);
                }
                Interaction::Hovered => *bg = BackgroundColor(COLOR_BTN_HOVER),
                Interaction::None => *bg = BackgroundColor(COLOR_BTN),
            },
            MapHubButton::EditMap(id) => match *interaction {
                Interaction::Pressed => {
                    if let Some(map) = registry.maps.get(id) {
                        state.editing_map_id = Some(id.clone());
                        state.current_spec = map.clone();
                        next_state.set(GamePhase::EditMap);
                    }
                }
                Interaction::Hovered => *bg = BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                Interaction::None => *bg = BackgroundColor(Color::NONE),
            },
            MapHubButton::DeleteMap(id) => match *interaction {
                Interaction::Pressed => {
                    if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                        let _ = repo.delete_map_sync(&rt.0, id);
                        registry.maps.remove(id);
                    }
                    next_state.set(GamePhase::DesignMapHub);
                }
                Interaction::Hovered => *bg = BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                Interaction::None => *bg = BackgroundColor(Color::NONE),
            },
            MapHubButton::Back => match *interaction {
                Interaction::Pressed => {
                    next_state.set(GamePhase::DesignHub);
                }
                Interaction::Hovered => *bg = BackgroundColor(COLOR_BTN_HOVER),
                Interaction::None => *bg = BackgroundColor(COLOR_BTN),
            },
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// MAP EDITOR (EditMap)
// ═══════════════════════════════════════════════════════════════════════

fn spawn_map_editor(
    mut commands: Commands,
    state: Res<MapDesignState>,
) {
    let spec = &state.current_spec;
    let half_cells = (spec.arena_radius / GRID_CELL_SIZE).ceil() as i32;
    let grid_dim = half_cells * 2 + 1;
    // Cell pixel size: fit in ~600px container
    let cell_px = (600.0 / grid_dim as f32).floor().max(4.0).min(14.0);

    commands
        .spawn((
            MapScreenRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(COLOR_BG),
        ))
        .with_children(|outer| {
            // ── Top bar: inputs + buttons ──
            outer
                .spawn(Node {
                    padding: UiRect::all(Val::Px(16.0)),
                    column_gap: Val::Px(16.0),
                    align_items: AlignItems::Center,
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|bar| {
                    // Name field
                    bar.spawn((
                        Text::new("Name:"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_DIM),
                    ));
                    spawn_text_input(bar, "name", &spec.name);

                    // Radius field
                    bar.spawn((
                        Text::new("Radius:"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_DIM),
                    ));
                    spawn_text_input(bar, "radius", &format!("{}", spec.arena_radius));

                    // Save / Cancel
                    spawn_button(bar, "Save", MapEditorButton::Save);
                    spawn_button(bar, "Cancel", MapEditorButton::Cancel);
                });

            // ── Main area: tools + grid ──
            outer
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: Val::Px(0.0),
                    min_height: Val::Px(0.0),
                    flex_direction: FlexDirection::Row,
                    padding: UiRect::horizontal(Val::Px(16.0)),
                    column_gap: Val::Px(16.0),
                    ..default()
                })
                .with_children(|main_area| {
                    // ── Tool palette ──
                    main_area
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        })
                        .with_children(|tools| {
                            tools.spawn((
                                Text::new("Tools"),
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT),
                            ));

                            let tool_items = [
                                ToolSelection::Obstacle,
                                ToolSelection::GravityDevice,
                                ToolSelection::SpeedBoost,
                                ToolSelection::DamageBoost,
                                ToolSelection::Erase,
                            ];
                            for tool in tool_items {
                                let is_selected = state.selected_tool == tool;
                                let bg_color = if is_selected {
                                    COLOR_TOOL_SELECTED
                                } else {
                                    COLOR_BTN
                                };
                                tools
                                    .spawn((
                                        MapEditorButton::SelectTool(tool),
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border_radius: BorderRadius::all(Val::Px(4.0)),
                                            min_width: Val::Px(90.0),
                                            ..default()
                                        },
                                        BackgroundColor(bg_color),
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(tool.display_name()),
                                            TextFont {
                                                font_size: 15.0,
                                                ..default()
                                            },
                                            TextColor(COLOR_TEXT),
                                        ));
                                    });
                            }
                        });

                    // ── Grid area ──
                    main_area
                        .spawn((
                            Node {
                                flex_grow: 1.0,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .with_children(|center| {
                            center
                                .spawn((
                                    GridContainer,
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        ..default()
                                    },
                                ))
                                .with_children(|grid| {
                                    spawn_grid_cells(grid, spec, half_cells, cell_px);
                                });
                        });
                });

            // ── Status bar ──
            outer
                .spawn(Node {
                    padding: UiRect::all(Val::Px(12.0)),
                    ..default()
                })
                .with_children(|bar| {
                    bar.spawn((
                        StatusText,
                        Text::new(format!(
                            "Tool: {} | Grid: {}x{} | Click to place/remove",
                            state.selected_tool.display_name(),
                            grid_dim,
                            grid_dim
                        )),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_DIM),
                    ));
                });
        });
}

fn spawn_grid_cells(
    grid: &mut ChildSpawnerCommands,
    spec: &MapSpec,
    half_cells: i32,
    cell_px: f32,
) {
    // Iterate Y from +half_cells down to -half_cells so top of UI = +Y in game world
    for gy in ((-half_cells)..=half_cells).rev() {
        grid.spawn(Node {
            flex_direction: FlexDirection::Row,
            ..default()
        })
        .with_children(|row| {
            for gx in (-half_cells)..=half_cells {
                let valid = is_valid_placement(gx, gy, spec.arena_radius);
                let placed_item = spec
                    .placements
                    .iter()
                    .find(|p| p.grid_x == gx && p.grid_y == gy)
                    .map(|p| p.item);

                let cell_color = if let Some(item) = placed_item {
                    item.color()
                } else if valid {
                    COLOR_GRID_EMPTY
                } else {
                    COLOR_GRID_INVALID
                };

                row.spawn((
                    GridCell {
                        grid_x: gx,
                        grid_y: gy,
                    },
                    Button,
                    Node {
                        width: Val::Px(cell_px),
                        height: Val::Px(cell_px),
                        margin: UiRect::all(Val::Px(0.5)),
                        ..default()
                    },
                    BackgroundColor(cell_color),
                ));
            }
        });
    }
}

fn spawn_text_input(parent: &mut ChildSpawnerCommands, key: &str, default_value: &str) {
    parent
        .spawn((
            MapTextInput {
                value: default_value.to_string(),
                focused: false,
                field_key: key.to_string(),
            },
            Button,
            Node {
                width: Val::Px(140.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(COLOR_INPUT_BG),
        ))
        .with_children(|input| {
            input.spawn((
                MapTextInputDisplay,
                Text::new(default_value),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

fn map_text_input_system(
    mut inputs: Query<(
        &Interaction,
        &mut MapTextInput,
        &mut BackgroundColor,
        &Children,
    )>,
    mut displays: Query<&mut Text, With<MapTextInputDisplay>>,
    mut keyboard_events: MessageReader<KeyboardInput>,
) {
    // Focus on click
    for (interaction, mut input, mut bg, _) in &mut inputs {
        if *interaction == Interaction::Pressed {
            input.focused = true;
            *bg = BackgroundColor(COLOR_INPUT_FOCUS);
        }
    }

    let events: Vec<_> = keyboard_events.read().cloned().collect();

    for (_interaction, mut input, mut bg, children) in &mut inputs {
        if !input.focused {
            *bg = BackgroundColor(COLOR_INPUT_BG);
            continue;
        }
        *bg = BackgroundColor(COLOR_INPUT_FOCUS);

        for event in &events {
            if !event.state.is_pressed() {
                continue;
            }
            match &event.logical_key {
                Key::Backspace => {
                    input.value.pop();
                }
                Key::Escape | Key::Enter => {
                    input.focused = false;
                }
                Key::Character(c) => {
                    input.value.push_str(c.as_str());
                }
                _ => {}
            }
        }

        for child in children.iter() {
            if let Ok(mut text) = displays.get_mut(child) {
                **text = if input.value.is_empty() {
                    "...".into()
                } else {
                    input.value.clone()
                };
            }
        }
    }

    // Unfocus all others when one is clicked
    let any_clicked = inputs
        .iter()
        .any(|(i, _, _, _)| *i == Interaction::Pressed);
    if any_clicked {
        for (interaction, mut input, _, _) in &mut inputs {
            if *interaction != Interaction::Pressed {
                input.focused = false;
            }
        }
    }
}

fn read_input_field<F: bevy::ecs::query::QueryFilter>(
    inputs: &Query<(&Interaction, &mut MapTextInput, &mut BackgroundColor, &Children), F>,
    key: &str,
) -> String {
    for (_, input, _, _) in inputs.iter() {
        if input.field_key == key {
            return input.value.clone();
        }
    }
    String::new()
}

fn map_editor_system(
    mut grid_q: Query<
        (&Interaction, &GridCell, &mut BackgroundColor),
        Without<MapEditorButton>,
    >,
    mut btn_q: Query<
        (&Interaction, &MapEditorButton, &mut BackgroundColor),
        Without<GridCell>,
    >,
    inputs: Query<(&Interaction, &mut MapTextInput, &mut BackgroundColor, &Children), (Without<GridCell>, Without<MapEditorButton>)>,
    mut status_q: Query<&mut Text, With<StatusText>>,
    mut state: ResMut<MapDesignState>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut registry: ResMut<PartRegistry>,
    repo: Option<Res<SqliteRepo>>,
    rt: Option<Res<TokioRuntime>>,
) {
    // Handle grid cell clicks
    for (interaction, cell, mut bg) in &mut grid_q {
        let valid = is_valid_placement(cell.grid_x, cell.grid_y, state.current_spec.arena_radius);

        match *interaction {
            Interaction::Pressed => {
                if !valid {
                    continue;
                }

                // Remove any existing placement at this cell
                state
                    .current_spec
                    .placements
                    .retain(|p| p.grid_x != cell.grid_x || p.grid_y != cell.grid_y);

                // Place new item (unless erasing)
                if let Some(item) = state.selected_tool.to_map_item() {
                    state.current_spec.placements.push(MapPlacement {
                        grid_x: cell.grid_x,
                        grid_y: cell.grid_y,
                        item,
                    });
                    *bg = BackgroundColor(item.color());
                } else {
                    *bg = BackgroundColor(COLOR_GRID_EMPTY);
                }
            }
            Interaction::Hovered => {
                if valid {
                    // Only change hover if not already colored by placement
                    let has_placement = state
                        .current_spec
                        .placements
                        .iter()
                        .any(|p| p.grid_x == cell.grid_x && p.grid_y == cell.grid_y);
                    if !has_placement {
                        *bg = BackgroundColor(COLOR_GRID_HOVER);
                    }
                }
            }
            Interaction::None => {
                let placed = state
                    .current_spec
                    .placements
                    .iter()
                    .find(|p| p.grid_x == cell.grid_x && p.grid_y == cell.grid_y)
                    .map(|p| p.item);
                let color = if let Some(item) = placed {
                    item.color()
                } else if valid {
                    COLOR_GRID_EMPTY
                } else {
                    COLOR_GRID_INVALID
                };
                *bg = BackgroundColor(color);
            }
        }
    }

    // Handle editor buttons
    for (interaction, button, mut bg) in &mut btn_q {
        match button {
            MapEditorButton::Save => match *interaction {
                Interaction::Pressed => {
                    // Read name and radius from inputs
                    let name = read_input_field(&inputs, "name");
                    let radius_str = read_input_field(&inputs, "radius");
                    let radius = radius_str.parse::<f32>().unwrap_or(12.0).clamp(6.0, 24.0);

                    state.current_spec.name = if name.is_empty() {
                        "Unnamed Map".into()
                    } else {
                        name
                    };
                    state.current_spec.arena_radius = radius;

                    // Remove placements outside new radius
                    state
                        .current_spec
                        .placements
                        .retain(|p| is_valid_placement(p.grid_x, p.grid_y, radius));

                    // Save to DB
                    if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                        let placements_json =
                            serde_json::to_string(&state.current_spec.placements)
                                .unwrap_or_else(|_| "[]".into());
                        let _ = repo.save_map_sync(
                            &rt.0,
                            &state.current_spec.id,
                            &state.current_spec.name,
                            state.current_spec.arena_radius,
                            &placements_json,
                        );
                    }

                    // Update registry
                    registry
                        .maps
                        .insert(state.current_spec.id.clone(), state.current_spec.clone());

                    next_state.set(GamePhase::DesignMapHub);
                }
                Interaction::Hovered => *bg = BackgroundColor(COLOR_BTN_HOVER),
                Interaction::None => *bg = BackgroundColor(COLOR_BTN),
            },
            MapEditorButton::Cancel => match *interaction {
                Interaction::Pressed => {
                    next_state.set(GamePhase::DesignMapHub);
                }
                Interaction::Hovered => *bg = BackgroundColor(COLOR_BTN_HOVER),
                Interaction::None => *bg = BackgroundColor(COLOR_BTN),
            },
            MapEditorButton::SelectTool(tool) => match *interaction {
                Interaction::Pressed => {
                    state.selected_tool = *tool;
                    // Update status text
                    if let Ok(mut status) = status_q.single_mut() {
                        **status = format!(
                            "Tool: {} | Click to place/remove",
                            tool.display_name()
                        );
                    }
                    *bg = BackgroundColor(COLOR_TOOL_SELECTED);
                }
                Interaction::Hovered => {
                    if state.selected_tool != *tool {
                        *bg = BackgroundColor(COLOR_BTN_HOVER);
                    }
                }
                Interaction::None => {
                    if state.selected_tool == *tool {
                        *bg = BackgroundColor(COLOR_TOOL_SELECTED);
                    } else {
                        *bg = BackgroundColor(COLOR_BTN);
                    }
                }
            },
        }
    }
}
