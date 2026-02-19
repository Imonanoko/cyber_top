use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

use crate::assets_map::GameAssets;
use crate::game::components::GamePhase;
use crate::game::parts::registry::PartRegistry;

// ── Data types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    PvP,
    PvAI,
}

#[derive(Resource)]
pub struct GameSelection {
    pub mode: GameMode,
    pub map_id: String,
    pub p1_build_id: String,
    pub p2_build_id: String,
}

impl Default for GameSelection {
    fn default() -> Self {
        Self {
            mode: GameMode::PvAI,
            map_id: "default_arena".into(),
            p1_build_id: "default_blaster".into(),
            p2_build_id: "default_blade".into(),
        }
    }
}

/// Tracks which player is currently picking in the PickTop screen.
#[derive(Resource, Default)]
pub struct PickingFor(pub u8); // 1 = P1, 2 = P2

// ── Marker components ────────────────────────────────────────────────

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct SelectionRoot;

#[derive(Component)]
struct PickerRoot;

#[derive(Component)]
struct GameOverOverlay;

#[derive(Component)]
enum MenuButton {
    StartGame,
    DesignMap,
    DesignTop,
}

#[derive(Component)]
enum SelectionButton {
    ModePvP,
    ModePvAI,
    ChooseMap,
    ChooseP1Top,
    ChooseP2Top,
    StartBattle,
    Back,
}

#[derive(Component)]
struct SelectionHighlight;

#[derive(Component)]
struct P2Section;

#[derive(Component)]
struct P2AiLabel;

#[derive(Component)]
struct P2ChoosePanel;

/// Label showing current map/build name on the hub.
#[derive(Component)]
struct CurrentMapLabel;
#[derive(Component)]
struct CurrentP1BuildLabel;

// Picker screen buttons
#[derive(Component)]
enum PickerButton {
    SelectMap(String),
    SelectBuild(String),
    Confirm,
    Back,
}

#[derive(Component)]
struct PickerHighlight;

/// Preview circle in picker (visual representation of a top).
#[derive(Component)]
struct PreviewCircle;

// ── Colors ───────────────────────────────────────────────────────────

const COLOR_BG: Color = Color::srgba(0.08, 0.08, 0.12, 1.0);
const COLOR_BTN: Color = Color::srgba(0.18, 0.20, 0.28, 1.0);
const COLOR_BTN_HOVER: Color = Color::srgba(0.28, 0.32, 0.42, 1.0);
const COLOR_BTN_PRESS: Color = Color::srgba(0.12, 0.14, 0.20, 1.0);
const COLOR_SELECTED: Color = Color::srgba(0.15, 0.45, 0.75, 1.0);
const COLOR_SELECTED_HOVER: Color = Color::srgba(0.20, 0.55, 0.85, 1.0);
const COLOR_TEXT: Color = Color::WHITE;
const COLOR_TEXT_DIM: Color = Color::srgba(0.5, 0.5, 0.5, 1.0);
const COLOR_ACCENT: Color = Color::srgba(0.2, 0.7, 1.0, 1.0);
const COLOR_CARD: Color = Color::srgba(0.12, 0.14, 0.20, 1.0);
const COLOR_CARD_SELECTED: Color = Color::srgba(0.15, 0.35, 0.60, 1.0);

// ── Plugin ───────────────────────────────────────────────────────────

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSelection>();
        app.init_resource::<PickingFor>();

        // Main menu
        app.add_systems(OnEnter(GamePhase::MainMenu), spawn_main_menu);
        app.add_systems(OnExit(GamePhase::MainMenu), despawn::<MainMenuRoot>);
        app.add_systems(Update, menu_button_system.run_if(in_state(GamePhase::MainMenu)));

        // Selection hub
        app.add_systems(OnEnter(GamePhase::Selection), spawn_selection_hub);
        app.add_systems(OnExit(GamePhase::Selection), despawn::<SelectionRoot>);
        app.add_systems(
            Update,
            (selection_button_system, update_selection_hub_visuals)
                .chain()
                .run_if(in_state(GamePhase::Selection)),
        );

        // Map picker
        app.add_systems(OnEnter(GamePhase::PickMap), spawn_map_picker);
        app.add_systems(OnExit(GamePhase::PickMap), despawn::<PickerRoot>);
        app.add_systems(Update, map_picker_system.run_if(in_state(GamePhase::PickMap)));

        // Top picker
        app.add_systems(OnEnter(GamePhase::PickTop), spawn_top_picker);
        app.add_systems(OnExit(GamePhase::PickTop), despawn::<PickerRoot>);
        app.add_systems(
            Update,
            (top_picker_system, update_top_picker_visuals)
                .chain()
                .run_if(in_state(GamePhase::PickTop)),
        );

        // Game over overlay
        app.add_systems(OnEnter(GamePhase::GameOver), spawn_game_over_overlay);
        app.add_systems(OnExit(GamePhase::GameOver), despawn::<GameOverOverlay>);
        app.add_systems(Update, game_over_input.run_if(in_state(GamePhase::GameOver)));
    }
}

// ── Generic despawn ──────────────────────────────────────────────────

fn despawn<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

// ═══════════════════════════════════════════════════════════════════════
// MAIN MENU
// ═══════════════════════════════════════════════════════════════════════

fn spawn_main_menu(mut commands: Commands) {
    commands
        .spawn((
            MainMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(COLOR_BG),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("CYBER TOP"),
                TextFont { font_size: 64.0, ..default() },
                TextColor(COLOR_ACCENT),
                Node { margin: UiRect::bottom(Val::Px(40.0)), ..default() },
            ));
            spawn_btn(parent, "Start Game", MenuButton::StartGame, COLOR_BTN, COLOR_TEXT, 360.0, 56.0);
            spawn_btn(parent, "Design Map", MenuButton::DesignMap, COLOR_BTN, COLOR_TEXT, 360.0, 56.0);
            spawn_btn(parent, "Design Top", MenuButton::DesignTop, COLOR_BTN, COLOR_TEXT, 360.0, 56.0);
        });
}

fn menu_button_system(
    mut q: Query<(&Interaction, &MenuButton, &mut BackgroundColor), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    for (interaction, button, mut bg) in &mut q {
        match button {
            MenuButton::StartGame => match *interaction {
                Interaction::Pressed => {
                    *bg = BackgroundColor(COLOR_BTN_PRESS);
                    next_state.set(GamePhase::Selection);
                }
                Interaction::Hovered => *bg = BackgroundColor(COLOR_BTN_HOVER),
                Interaction::None => *bg = BackgroundColor(COLOR_BTN),
            },
            MenuButton::DesignTop => match *interaction {
                Interaction::Pressed => {
                    *bg = BackgroundColor(COLOR_BTN_PRESS);
                    next_state.set(GamePhase::DesignHub);
                }
                Interaction::Hovered => *bg = BackgroundColor(COLOR_BTN_HOVER),
                Interaction::None => *bg = BackgroundColor(COLOR_BTN),
            },
            MenuButton::DesignMap => match *interaction {
                Interaction::Pressed => {
                    *bg = BackgroundColor(COLOR_BTN_PRESS);
                    next_state.set(GamePhase::DesignMapHub);
                }
                Interaction::Hovered => *bg = BackgroundColor(COLOR_BTN_HOVER),
                Interaction::None => *bg = BackgroundColor(COLOR_BTN),
            },
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SELECTION HUB
// ═══════════════════════════════════════════════════════════════════════

fn spawn_selection_hub(mut commands: Commands, selection: Res<GameSelection>, registry: Res<PartRegistry>) {
    let p1_name = build_display_name(&selection.p1_build_id, &registry);
    let p2_name = build_display_name(&selection.p2_build_id, &registry);

    commands
        .spawn((
            SelectionRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                padding: UiRect::new(Val::Px(30.0), Val::Px(30.0), Val::Px(40.0), Val::Px(30.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            BackgroundColor(COLOR_BG),
        ))
        .with_children(|root| {
            // Title
            root.spawn((
                Text::new("Game Setup"),
                TextFont { font_size: 40.0, ..default() },
                TextColor(COLOR_ACCENT),
                Node { margin: UiRect::bottom(Val::Px(16.0)), ..default() },
            ));

            // ── Mode ──
            section_label(root, "Mode");
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                ..default()
            }).with_children(|row| {
                spawn_sel_btn(row, "Player vs AI", SelectionButton::ModePvAI,
                    selection.mode == GameMode::PvAI);
                spawn_sel_btn(row, "Player vs Player", SelectionButton::ModePvP,
                    selection.mode == GameMode::PvP);
            });

            // ── Map ──
            section_label(root, "Map");
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                align_items: AlignItems::Center,
                ..default()
            }).with_children(|row| {
                row.spawn((
                    CurrentMapLabel,
                    Text::new(map_display_name(&selection.map_id)),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(COLOR_TEXT),
                    Node { margin: UiRect::right(Val::Px(12.0)), ..default() },
                ));
                spawn_sel_btn(row, "Choose...", SelectionButton::ChooseMap, false);
            });

            // ── Player 1 ──
            section_label(root, "Player 1 Build");
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                align_items: AlignItems::Center,
                ..default()
            }).with_children(|row| {
                row.spawn((
                    CurrentP1BuildLabel,
                    Text::new(p1_name),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(COLOR_TEXT),
                    Node { margin: UiRect::right(Val::Px(12.0)), ..default() },
                ));
                spawn_sel_btn(row, "Choose...", SelectionButton::ChooseP1Top, false);
            });

            // ── Player 2 ──
            root.spawn((
                P2Section,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
            )).with_children(|p2| {
                // AI label
                p2.spawn((
                    P2AiLabel,
                    Text::new("Player 2: AI (Random)"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(COLOR_TEXT_DIM),
                    Node {
                        display: if selection.mode == GameMode::PvAI { Display::Flex } else { Display::None },
                        ..default()
                    },
                ));
                // PvP choose
                p2.spawn((
                    P2ChoosePanel,
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        align_items: AlignItems::Center,
                        display: if selection.mode == GameMode::PvP { Display::Flex } else { Display::None },
                        ..default()
                    },
                )).with_children(|row| {
                    section_label(row, "Player 2 Build");
                    row.spawn((
                        Text::new(p2_name),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(COLOR_TEXT),
                        Node { margin: UiRect::right(Val::Px(12.0)), ..default() },
                    ));
                    spawn_sel_btn(row, "Choose...", SelectionButton::ChooseP2Top, false);
                });
            });

            // ── Action buttons ──
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(20.0),
                margin: UiRect::top(Val::Px(20.0)),
                ..default()
            }).with_children(|row| {
                spawn_sel_btn(row, "Back", SelectionButton::Back, false);
                spawn_sel_btn(row, "Start Battle!", SelectionButton::StartBattle, false);
            });
        });
}

fn selection_button_system(
    mut q: Query<(&Interaction, &SelectionButton, &mut BackgroundColor), Changed<Interaction>>,
    mut selection: ResMut<GameSelection>,
    mut picking: ResMut<PickingFor>,
    mut next_state: ResMut<NextState<GamePhase>>,
    registry: Res<PartRegistry>,
) {
    let build_ids: Vec<String> = registry.builds.keys().cloned().collect();
    for (interaction, button, _bg) in &mut q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            SelectionButton::ModePvP => selection.mode = GameMode::PvP,
            SelectionButton::ModePvAI => {
                selection.mode = GameMode::PvAI;
                randomize_ai_selection(&mut selection, &build_ids);
            }
            SelectionButton::ChooseMap => {
                next_state.set(GamePhase::PickMap);
            }
            SelectionButton::ChooseP1Top => {
                picking.0 = 1;
                next_state.set(GamePhase::PickTop);
            }
            SelectionButton::ChooseP2Top => {
                picking.0 = 2;
                next_state.set(GamePhase::PickTop);
            }
            SelectionButton::StartBattle => {
                if selection.mode == GameMode::PvAI {
                    randomize_ai_selection(&mut selection, &build_ids);
                }
                next_state.set(GamePhase::Aiming);
            }
            SelectionButton::Back => {
                next_state.set(GamePhase::MainMenu);
            }
        }
    }
}

fn update_selection_hub_visuals(
    selection: Res<GameSelection>,
    mut mode_btns: Query<(&SelectionButton, &Interaction, &mut BackgroundColor), With<SelectionHighlight>>,
    mut ai_label: Query<&mut Node, (With<P2AiLabel>, Without<P2ChoosePanel>)>,
    mut p2_panel: Query<&mut Node, (With<P2ChoosePanel>, Without<P2AiLabel>)>,
) {
    for mut node in &mut ai_label {
        node.display = if selection.mode == GameMode::PvAI { Display::Flex } else { Display::None };
    }
    for mut node in &mut p2_panel {
        node.display = if selection.mode == GameMode::PvP { Display::Flex } else { Display::None };
    }
    for (button, interaction, mut bg) in &mut mode_btns {
        let is_selected = match button {
            SelectionButton::ModePvP => selection.mode == GameMode::PvP,
            SelectionButton::ModePvAI => selection.mode == GameMode::PvAI,
            _ => false,
        };
        *bg = BackgroundColor(match (is_selected, interaction) {
            (true, Interaction::Hovered) => COLOR_SELECTED_HOVER,
            (true, _) => COLOR_SELECTED,
            (false, Interaction::Hovered) => COLOR_BTN_HOVER,
            (false, Interaction::Pressed) => COLOR_BTN_PRESS,
            (false, Interaction::None) => COLOR_BTN,
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// MAP PICKER
// ═══════════════════════════════════════════════════════════════════════

fn spawn_map_picker(mut commands: Commands, selection: Res<GameSelection>, registry: Res<PartRegistry>) {
    commands
        .spawn((
            PickerRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(40.0)),
                row_gap: Val::Px(20.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            BackgroundColor(COLOR_BG),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Select Map"),
                TextFont { font_size: 40.0, ..default() },
                TextColor(COLOR_ACCENT),
            ));

            // Scrollable card area
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(20.0),
                row_gap: Val::Px(20.0),
                margin: UiRect::top(Val::Px(20.0)),
                ..default()
            }).with_children(|grid| {
                let mut maps: Vec<_> = registry.maps.values().collect();
                maps.sort_by(|a, b| a.name.cmp(&b.name));
                for map in maps {
                    let desc = format!("R={:.0}, {} items", map.arena_radius, map.placements.len());
                    spawn_map_card(grid, &map.id, &map.name,
                        &desc, Color::srgba(0.15, 0.15, 0.2, 1.0),
                        selection.map_id == map.id);
                }
            });

            // Back button
            root.spawn(Node {
                margin: UiRect::top(Val::Px(20.0)),
                ..default()
            }).with_children(|row| {
                spawn_picker_btn(row, "Back", PickerButton::Back, false);
            });
        });
}

fn spawn_map_card(
    parent: &mut ChildSpawnerCommands,
    id: &str,
    name: &str,
    description: &str,
    preview_color: Color,
    selected: bool,
) {
    let card_bg = if selected { COLOR_CARD_SELECTED } else { COLOR_CARD };
    parent.spawn((
        PickerButton::SelectMap(id.into()),
        PickerHighlight,
        Button,
        Node {
            width: Val::Px(200.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(10.0),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(card_bg),
    )).with_children(|card| {
        // Preview: colored circle
        card.spawn((
            Node {
                width: Val::Px(120.0),
                height: Val::Px(120.0),
                border_radius: BorderRadius::all(Val::Px(60.0)),
                ..default()
            },
            BackgroundColor(preview_color),
        ));
        // Name
        card.spawn((
            Text::new(name),
            TextFont { font_size: 20.0, ..default() },
            TextColor(COLOR_TEXT),
        ));
        // Description
        card.spawn((
            Text::new(description),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_TEXT_DIM),
        ));
    });
}

fn map_picker_system(
    mut q: Query<(&Interaction, &PickerButton, &mut BackgroundColor), Changed<Interaction>>,
    mut selection: ResMut<GameSelection>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    for (interaction, button, mut bg) in &mut q {
        match button {
            PickerButton::SelectMap(id) => match *interaction {
                Interaction::Pressed => {
                    selection.map_id = id.clone();
                    next_state.set(GamePhase::Selection);
                }
                Interaction::Hovered => {
                    let is_sel = selection.map_id == *id;
                    *bg = BackgroundColor(if is_sel { COLOR_SELECTED_HOVER } else { COLOR_BTN_HOVER });
                }
                Interaction::None => {
                    let is_sel = selection.map_id == *id;
                    *bg = BackgroundColor(if is_sel { COLOR_CARD_SELECTED } else { COLOR_CARD });
                }
            },
            PickerButton::Back => {
                if *interaction == Interaction::Pressed {
                    next_state.set(GamePhase::Selection);
                }
            }
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// BUILD PICKER
// ═══════════════════════════════════════════════════════════════════════

fn spawn_top_picker(
    mut commands: Commands,
    selection: Res<GameSelection>,
    picking: Res<PickingFor>,
    registry: Res<PartRegistry>,
    game_assets: Option<Res<GameAssets>>,
) {
    let player = picking.0;
    let cur_build = if player == 1 {
        &selection.p1_build_id
    } else {
        &selection.p2_build_id
    };

    commands
        .spawn((
            PickerRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(30.0)),
                row_gap: Val::Px(16.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            BackgroundColor(COLOR_BG),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!("Player {} - Select Build", player)),
                TextFont { font_size: 36.0, ..default() },
                TextColor(COLOR_ACCENT),
            ));

            // ── Build cards ──
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(16.0),
                row_gap: Val::Px(16.0),
                margin: UiRect::top(Val::Px(16.0)),
                ..default()
            }).with_children(|grid| {
                let mut build_ids: Vec<_> = registry.builds.keys().collect();
                build_ids.sort();
                for id in build_ids {
                    let build_ref = &registry.builds[id];
                    let top_sprite = game_assets.as_ref()
                        .and_then(|a| a.top_sprites.get(build_ref.top_id.as_str()).cloned());
                    let weapon_name = registry.weapons.get(&build_ref.weapon_id)
                        .map(|w| format!("{:?}", w.kind))
                        .unwrap_or_default();
                    spawn_build_card(grid, id, &build_ref.name, &weapon_name, *cur_build == *id, top_sprite);
                }
            });

            // ── Confirm / Back ──
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(20.0),
                margin: UiRect::top(Val::Px(16.0)),
                ..default()
            }).with_children(|row| {
                spawn_picker_btn(row, "Back", PickerButton::Back, false);
                spawn_picker_btn(row, "Confirm", PickerButton::Confirm, false);
            });
        });
}

fn spawn_build_card(
    parent: &mut ChildSpawnerCommands,
    id: &str,
    name: &str,
    weapon_kind: &str,
    selected: bool,
    top_sprite: Option<Handle<Image>>,
) {
    let card_bg = if selected { COLOR_CARD_SELECTED } else { COLOR_CARD };

    parent.spawn((
        PickerButton::SelectBuild(id.into()),
        PickerHighlight,
        Button,
        Node {
            width: Val::Px(200.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(14.0)),
            row_gap: Val::Px(8.0),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(card_bg),
    )).with_children(|card| {
        // Top preview
        if let Some(handle) = top_sprite {
            card.spawn((
                PreviewCircle,
                ImageNode { image: handle, ..default() },
                Node {
                    width: Val::Px(80.0),
                    height: Val::Px(80.0),
                    ..default()
                },
            ));
        } else {
            card.spawn((
                PreviewCircle,
                Node {
                    width: Val::Px(80.0),
                    height: Val::Px(80.0),
                    border_radius: BorderRadius::all(Val::Px(40.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.6, 1.0)),
            ));
        }
        // Build name
        card.spawn((
            Text::new(name),
            TextFont { font_size: 18.0, ..default() },
            TextColor(COLOR_TEXT),
        ));
        // Weapon kind
        card.spawn((
            Text::new(weapon_kind),
            TextFont { font_size: 13.0, ..default() },
            TextColor(COLOR_ACCENT),
        ));
    });
}

fn top_picker_system(
    mut q: Query<(&Interaction, &PickerButton), Changed<Interaction>>,
    mut selection: ResMut<GameSelection>,
    picking: Res<PickingFor>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    let player = picking.0;
    for (interaction, button) in &mut q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            PickerButton::SelectBuild(id) => {
                if player == 1 {
                    selection.p1_build_id = id.clone();
                } else {
                    selection.p2_build_id = id.clone();
                }
            }
            PickerButton::Confirm | PickerButton::Back => {
                next_state.set(GamePhase::Selection);
            }
            _ => {}
        }
    }
}

fn update_top_picker_visuals(
    selection: Res<GameSelection>,
    picking: Res<PickingFor>,
    mut q: Query<(&PickerButton, &Interaction, &mut BackgroundColor), With<PickerHighlight>>,
) {
    let cur_build = if picking.0 == 1 {
        &selection.p1_build_id
    } else {
        &selection.p2_build_id
    };

    for (button, interaction, mut bg) in &mut q {
        let is_selected = match button {
            PickerButton::SelectBuild(id) => *cur_build == *id,
            _ => false,
        };
        *bg = BackgroundColor(match (is_selected, interaction) {
            (true, Interaction::Hovered) => COLOR_SELECTED_HOVER,
            (true, _) => COLOR_CARD_SELECTED,
            (false, Interaction::Hovered) => COLOR_BTN_HOVER,
            (false, Interaction::Pressed) => COLOR_BTN_PRESS,
            (false, Interaction::None) => COLOR_CARD,
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// GAME OVER OVERLAY
// ═══════════════════════════════════════════════════════════════════════

fn spawn_game_over_overlay(
    mut commands: Commands,
    player: Query<&crate::game::components::SpinHpCurrent, With<crate::game::components::PlayerControlled>>,
    ai: Query<
        &crate::game::components::SpinHpCurrent,
        (With<crate::game::components::AiControlled>, Without<crate::game::components::PlayerControlled>),
    >,
    p2: Query<
        &crate::game::components::SpinHpCurrent,
        (
            With<crate::game::components::Player2Controlled>,
            Without<crate::game::components::PlayerControlled>,
            Without<crate::game::components::AiControlled>,
        ),
    >,
) {
    let player_hp = player.iter().next().map(|s| s.0 .0).unwrap_or(0.0);
    let opponent_hp = ai.iter().next().or_else(|| p2.iter().next()).map(|s| s.0 .0).unwrap_or(0.0);
    let winner = if player_hp > opponent_hp { "Player 1 Wins!" } else { "Player 2 Wins!" };

    commands
        .spawn((
            GameOverOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(24.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(10),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(winner),
                TextFont { font_size: 56.0, ..default() },
                TextColor(Color::srgb(1.0, 1.0, 0.0)),
            ));
            parent.spawn((
                Text::new("Press ESCAPE to return to menu"),
                TextFont { font_size: 22.0, ..default() },
                TextColor(COLOR_TEXT_DIM),
            ));
        });
}

fn game_over_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) || keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(GamePhase::MainMenu);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════

fn randomize_ai_selection(selection: &mut GameSelection, build_ids: &[String]) {
    if build_ids.is_empty() {
        return;
    }
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    selection.p2_build_id = build_ids[(nanos as usize) % build_ids.len()].clone();
}

fn map_display_name(id: &str) -> &str {
    match id {
        "default_arena" => "Default Arena",
        _ => id,
    }
}

fn build_display_name(id: &str, registry: &PartRegistry) -> String {
    registry.builds.get(id)
        .map(|b| b.name.clone())
        .unwrap_or_else(|| id.to_string())
}

/// Generic button spawner for menu screens.
fn spawn_btn<C: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: C,
    bg_color: Color,
    text_color: Color,
    width: f32,
    height: f32,
) {
    parent.spawn((
        marker,
        Button,
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(bg_color),
    )).with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont { font_size: 24.0, ..default() },
            TextColor(text_color),
        ));
    });
}

fn section_label(parent: &mut ChildSpawnerCommands, label: &str) {
    parent.spawn((
        Text::new(label),
        TextFont { font_size: 20.0, ..default() },
        TextColor(COLOR_TEXT_DIM),
        Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
    ));
}

/// Selection-hub button (smaller).
fn spawn_sel_btn(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: SelectionButton,
    selected: bool,
) {
    let bg = if selected { COLOR_SELECTED } else { COLOR_BTN };
    parent.spawn((
        marker,
        SelectionHighlight,
        Button,
        Node {
            min_width: Val::Px(140.0),
            height: Val::Px(40.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(14.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(bg),
    )).with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont { font_size: 18.0, ..default() },
            TextColor(COLOR_TEXT),
        ));
    });
}

/// Picker-screen button.
fn spawn_picker_btn(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    marker: PickerButton,
    selected: bool,
) {
    let bg = if selected { COLOR_SELECTED } else { COLOR_BTN };
    parent.spawn((
        marker,
        Button,
        Node {
            min_width: Val::Px(140.0),
            height: Val::Px(44.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(16.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(bg),
    )).with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont { font_size: 20.0, ..default() },
            TextColor(COLOR_TEXT),
        ));
    });
}
