use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use std::time::SystemTime;

use crate::config::tuning::Tuning;
use crate::game::components::GamePhase;
use crate::game::parts::registry::PartRegistry;
use crate::game::parts::weapon_wheel::{MeleeSpec, RangedSpec, WeaponWheelSpec};
use crate::game::parts::shaft::ShaftSpec;
use crate::game::parts::chassis::ChassisSpec;
use crate::game::parts::trait_screw::TraitScrewSpec;
use crate::game::stats::base::BaseStats;
use crate::game::stats::types::{MetersPerSec, PartSlot, Radius, SpinHp, WeaponKind};
use crate::plugins::storage_plugin::TokioRuntime;
use crate::storage::sqlite_repo::SqliteRepo;

// ── Colors (match menu_plugin style) ────────────────────────────────

const COLOR_BG: Color = Color::srgba(0.08, 0.08, 0.12, 1.0);
const COLOR_BTN: Color = Color::srgba(0.18, 0.20, 0.28, 1.0);
const COLOR_BTN_HOVER: Color = Color::srgba(0.28, 0.32, 0.42, 1.0);
const COLOR_TEXT: Color = Color::WHITE;
const COLOR_TEXT_DIM: Color = Color::srgba(0.5, 0.5, 0.5, 1.0);
const COLOR_ACCENT: Color = Color::srgba(0.2, 0.7, 1.0, 1.0);
const COLOR_CARD: Color = Color::srgba(0.12, 0.14, 0.20, 1.0);
const COLOR_CARD_SELECTED: Color = Color::srgba(0.15, 0.35, 0.60, 1.0);
const COLOR_INPUT_BG: Color = Color::srgba(0.10, 0.10, 0.16, 1.0);
const COLOR_INPUT_FOCUS: Color = Color::srgba(0.15, 0.15, 0.25, 1.0);

// ── Plugin ──────────────────────────────────────────────────────────

pub struct DesignPlugin;

impl Plugin for DesignPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DesignState>();

        // DesignHub
        app.add_systems(OnEnter(GamePhase::DesignHub), spawn_design_hub);
        app.add_systems(OnExit(GamePhase::DesignHub), despawn::<ScreenRoot>);
        app.add_systems(Update, design_hub_system.run_if(in_state(GamePhase::DesignHub)));

        // ManageParts
        app.add_systems(OnEnter(GamePhase::ManageParts), spawn_manage_parts);
        app.add_systems(OnExit(GamePhase::ManageParts), despawn::<ScreenRoot>);
        app.add_systems(Update, manage_parts_system.run_if(in_state(GamePhase::ManageParts)));

        // EditWheel
        app.add_systems(OnEnter(GamePhase::EditWheel), spawn_wheel_editor);
        app.add_systems(OnExit(GamePhase::EditWheel), despawn::<ScreenRoot>);
        app.add_systems(Update, (text_input_system, wheel_editor_system).chain().run_if(in_state(GamePhase::EditWheel)));

        // EditShaft
        app.add_systems(OnEnter(GamePhase::EditShaft), spawn_shaft_editor);
        app.add_systems(OnExit(GamePhase::EditShaft), despawn::<ScreenRoot>);
        app.add_systems(Update, (text_input_system, shaft_editor_system).chain().run_if(in_state(GamePhase::EditShaft)));

        // EditChassis
        app.add_systems(OnEnter(GamePhase::EditChassis), spawn_chassis_editor);
        app.add_systems(OnExit(GamePhase::EditChassis), despawn::<ScreenRoot>);
        app.add_systems(Update, (text_input_system, chassis_editor_system).chain().run_if(in_state(GamePhase::EditChassis)));

        // EditScrew
        app.add_systems(OnEnter(GamePhase::EditScrew), spawn_screw_editor);
        app.add_systems(OnExit(GamePhase::EditScrew), despawn::<ScreenRoot>);
        app.add_systems(Update, (text_input_system, screw_editor_system).chain().run_if(in_state(GamePhase::EditScrew)));

        // EditWeapon
        app.add_systems(OnEnter(GamePhase::EditWeapon), spawn_weapon_editor);
        app.add_systems(OnExit(GamePhase::EditWeapon), despawn::<ScreenRoot>);
        app.add_systems(Update, (text_input_system, weapon_editor_system).chain().run_if(in_state(GamePhase::EditWeapon)));

        // AssembleBuild
        app.add_systems(OnEnter(GamePhase::AssembleBuild), spawn_assemble_build);
        app.add_systems(OnExit(GamePhase::AssembleBuild), despawn::<ScreenRoot>);
        app.add_systems(Update, (text_input_system, assemble_build_system).chain().run_if(in_state(GamePhase::AssembleBuild)));

        // PickDesignPart
        app.add_systems(OnEnter(GamePhase::PickDesignPart), spawn_pick_design_part);
        app.add_systems(OnExit(GamePhase::PickDesignPart), despawn::<ScreenRoot>);
        app.add_systems(Update, pick_design_part_system.run_if(in_state(GamePhase::PickDesignPart)));

        // Global UI scroll (works for all scroll containers across all screens)
        app.add_systems(Update, ui_scroll_system);
    }
}

// ── UI Scroll System ────────────────────────────────────────────────

const SCROLL_LINE_HEIGHT: f32 = 21.0;

fn ui_scroll_system(
    mut mouse_wheel: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut scroll_q: Query<&mut ScrollPosition>,
) {
    for ev in mouse_wheel.read() {
        let mut dy = -ev.y;
        if ev.unit == MouseScrollUnit::Line {
            dy *= SCROLL_LINE_HEIGHT;
        }

        for pointer_map in hover_map.values() {
            for &entity in pointer_map.keys() {
                if let Ok(mut scroll) = scroll_q.get_mut(entity) {
                    scroll.y = (scroll.y + dy).max(0.0);
                }
            }
        }
    }
}

// ── Marker components ───────────────────────────────────────────────

#[derive(Component)]
struct ScreenRoot;

// ── Design State ────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct DesignState {
    /// Part ID being edited (None = creating new part)
    pub editing_part_id: Option<String>,
    /// Which slot we're picking for in PickDesignPart
    pub picking_slot: Option<PartSlot>,
    /// Build being assembled
    pub current_build_id: Option<String>,
    pub current_build_wheel_id: String,
    pub current_build_weapon_id: String,
    pub current_build_shaft_id: String,
    pub current_build_chassis_id: String,
    pub current_build_screw_id: String,
    pub current_build_note: String,
    /// Where to return after editor save (DesignHub for create, ManageParts for edit)
    pub return_to_manage: bool,
    /// Error message shown when a delete is blocked (e.g. part used by builds)
    pub delete_error: Option<String>,
}

// ── Text Input Widget ───────────────────────────────────────────────

#[derive(Component)]
struct TextInput {
    value: String,
    focused: bool,
    field_key: String,
}

#[derive(Component)]
struct TextInputDisplay;

fn text_input_system(
    mut inputs: Query<(&Interaction, &mut TextInput, &mut BackgroundColor, &Children)>,
    mut displays: Query<&mut Text, With<TextInputDisplay>>,
    mut keyboard_events: MessageReader<KeyboardInput>,
) {
    // Focus on click
    for (interaction, mut input, mut bg, _) in &mut inputs {
        if *interaction == Interaction::Pressed {
            input.focused = true;
            *bg = BackgroundColor(COLOR_INPUT_FOCUS);
        }
    }

    // Collect keyboard events
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

        // Update display text
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
    let any_clicked = inputs.iter().any(|(i, _, _, _)| *i == Interaction::Pressed);
    if any_clicked {
        for (interaction, mut input, _, _) in &mut inputs {
            if *interaction != Interaction::Pressed {
                input.focused = false;
            }
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
    format!("custom_{:08x}", nanos)
}

fn slot_dir(slot: &PartSlot) -> &'static str {
    match slot {
        PartSlot::WeaponWheel => "weapons",
        PartSlot::Shaft => "shafts",
        PartSlot::Chassis => "chassis",
        PartSlot::TraitScrew => "screws",
    }
}

fn is_builtin(id: &str) -> bool {
    matches!(
        id,
        "default_top" | "basic_blade" | "basic_blaster"
            | "standard_shaft" | "standard_chassis" | "standard_screw"
            | "default_blade" | "default_blaster"
    )
}

fn builds_using_part(registry: &PartRegistry, part_id: &str) -> Vec<String> {
    registry.builds.values()
        .filter(|b| {
            b.wheel_id == part_id
                || b.weapon_id == part_id
                || b.shaft_id == part_id
                || b.chassis_id == part_id
                || b.screw_id == part_id
        })
        .map(|b| b.name.clone())
        .collect()
}

fn spawn_title(parent: &mut ChildSpawnerCommands, title: &str) {
    parent.spawn((
        Text::new(title),
        TextFont { font_size: 36.0, ..default() },
        TextColor(COLOR_ACCENT),
        Node { margin: UiRect::bottom(Val::Px(16.0)), ..default() },
    ));
}

fn spawn_button<C: Component>(parent: &mut ChildSpawnerCommands, label: &str, marker: C) {
    parent.spawn((
        marker,
        Button,
        Node {
            min_width: Val::Px(160.0),
            height: Val::Px(44.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(16.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(COLOR_BTN),
    )).with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont { font_size: 18.0, ..default() },
            TextColor(COLOR_TEXT),
        ));
    });
}

fn spawn_field_row(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    description: &str,
    field_key: &str,
    default_value: &str,
) {
    parent.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        ..default()
    }).with_children(|row| {
        // Label + description
        row.spawn(Node {
            width: Val::Px(200.0),
            flex_direction: FlexDirection::Column,
            ..default()
        }).with_children(|col| {
            col.spawn((
                Text::new(label),
                TextFont { font_size: 16.0, ..default() },
                TextColor(COLOR_TEXT),
            ));
            col.spawn((
                Text::new(description),
                TextFont { font_size: 11.0, ..default() },
                TextColor(COLOR_TEXT_DIM),
            ));
        });

        // Text input
        row.spawn((
            TextInput {
                value: default_value.into(),
                focused: false,
                field_key: field_key.into(),
            },
            Button,
            Node {
                width: Val::Px(180.0),
                height: Val::Px(32.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(8.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(COLOR_INPUT_BG),
        )).with_children(|input| {
            input.spawn((
                TextInputDisplay,
                Text::new(if default_value.is_empty() { "..." } else { default_value }),
                TextFont { font_size: 15.0, ..default() },
                TextColor(COLOR_TEXT),
            ));
        });
    });
}

fn read_field(inputs: &Query<&TextInput>, key: &str) -> String {
    for input in inputs.iter() {
        if input.field_key == key {
            return input.value.clone();
        }
    }
    String::new()
}

fn read_f32(inputs: &Query<&TextInput>, key: &str, default: f32) -> f32 {
    read_field(inputs, key).parse().unwrap_or(default)
}

fn read_u32(inputs: &Query<&TextInput>, key: &str, default: u32) -> u32 {
    read_field(inputs, key).parse().unwrap_or(default)
}

fn hover_system(interaction: &Interaction, bg: &mut BackgroundColor) {
    match interaction {
        Interaction::Hovered => *bg = BackgroundColor(COLOR_BTN_HOVER),
        Interaction::None => *bg = BackgroundColor(COLOR_BTN),
        _ => {}
    }
}

fn spawn_image_preview(parent: &mut ChildSpawnerCommands, image: Option<Handle<Image>>, size: f32) {
    if let Some(handle) = image {
        parent.spawn((
            ImageNode { image: handle, ..default() },
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                ..default()
            },
        ));
    } else {
        parent.spawn((
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.15, 0.22, 1.0)),
        ));
    }
}

fn spawn_card_frame(
    parent: &mut ChildSpawnerCommands,
    name: &str,
    stats_line: &str,
    image: Option<Handle<Image>>,
    bg_color: Color,
    width: f32,
    spawn_extras: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent.spawn((
        Node {
            width: Val::Px(width),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(10.0)),
            row_gap: Val::Px(6.0),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(bg_color),
    )).with_children(|card| {
        spawn_image_preview(card, image, 64.0);
        card.spawn((
            Text::new(name),
            TextFont { font_size: 14.0, ..default() },
            TextColor(COLOR_TEXT),
        ));
        card.spawn((
            Text::new(stats_line),
            TextFont { font_size: 11.0, ..default() },
            TextColor(COLOR_TEXT_DIM),
        ));
        spawn_extras(card);
    });
}

// ═══════════════════════════════════════════════════════════════════════
// DESIGN HUB (Create entry point)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Component)]
enum HubButton {
    NewWheel,
    NewWeapon,
    NewShaft,
    NewChassis,
    NewScrew,
    ManageParts,
    DesignMap,
    Back,
}

fn spawn_design_hub(mut commands: Commands, mut state: ResMut<DesignState>) {
    state.editing_part_id = None;
    state.return_to_manage = false;

    commands.spawn((
        ScreenRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            padding: UiRect::new(Val::Px(30.0), Val::Px(30.0), Val::Px(40.0), Val::Px(30.0)),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        BackgroundColor(COLOR_BG),
    )).with_children(|root| {
        spawn_title(root, "Design Workshop");

        // Create section label
        root.spawn((
            Text::new("Create New Part"),
            TextFont { font_size: 20.0, ..default() },
            TextColor(COLOR_TEXT_DIM),
            Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
        ));

        // Create buttons (2x2 grid)
        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(16.0),
            row_gap: Val::Px(12.0),
            ..default()
        }).with_children(|grid| {
            spawn_button(grid, "New Wheel", HubButton::NewWheel);
            spawn_button(grid, "New Weapon", HubButton::NewWeapon);
            spawn_button(grid, "New Shaft", HubButton::NewShaft);
            spawn_button(grid, "New Chassis", HubButton::NewChassis);
            spawn_button(grid, "New Screw", HubButton::NewScrew);
        });

        // Manage section
        root.spawn(Node {
            margin: UiRect::top(Val::Px(24.0)),
            column_gap: Val::Px(16.0),
            ..default()
        }).with_children(|row| {
            spawn_button(row, "My Parts & Builds", HubButton::ManageParts);
            spawn_button(row, "Design Map", HubButton::DesignMap);
        });

        // Back
        root.spawn(Node { margin: UiRect::top(Val::Px(12.0)), ..default() }).with_children(|row| {
            spawn_button(row, "Back", HubButton::Back);
        });
    });
}

fn design_hub_system(
    mut q: Query<(&Interaction, &HubButton, &mut BackgroundColor), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut state: ResMut<DesignState>,
) {
    for (interaction, button, mut bg) in &mut q {
        if *interaction == Interaction::Pressed {
            state.return_to_manage = false;
            match button {
                HubButton::NewWheel => {
                    state.editing_part_id = Some(gen_custom_id());
                    next_state.set(GamePhase::EditWheel);
                }
                HubButton::NewWeapon => {
                    state.editing_part_id = Some(gen_custom_id());
                    next_state.set(GamePhase::EditWeapon);
                }
                HubButton::NewShaft => {
                    state.editing_part_id = Some(gen_custom_id());
                    next_state.set(GamePhase::EditShaft);
                }
                HubButton::NewChassis => {
                    state.editing_part_id = Some(gen_custom_id());
                    next_state.set(GamePhase::EditChassis);
                }
                HubButton::NewScrew => {
                    state.editing_part_id = Some(gen_custom_id());
                    next_state.set(GamePhase::EditScrew);
                }
                HubButton::ManageParts => {
                    state.editing_part_id = None;
                    next_state.set(GamePhase::ManageParts);
                }
                HubButton::DesignMap => {
                    next_state.set(GamePhase::DesignMapHub);
                }
                HubButton::Back => {
                    state.editing_part_id = None;
                    next_state.set(GamePhase::MainMenu);
                }
            }
        }
        hover_system(interaction, &mut bg);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// MANAGE PARTS (Read / Update / Delete)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Component)]
enum ManageButton {
    EditWheel(String),
    DeleteWheel(String),
    EditPart { slot: PartSlot, id: String },
    DeletePart { slot: PartSlot, id: String },
    EditBuild(String),
    DeleteBuild(String),
    NewBuild,
    Back,
}

fn spawn_manage_parts(
    mut commands: Commands,
    registry: Res<PartRegistry>,
    asset_server: Res<AssetServer>,
    mut state: ResMut<DesignState>,
) {
    let error_msg = state.delete_error.take();
    let edit_icon: Handle<Image> = asset_server.load("ui/edit.png");
    let delete_icon: Handle<Image> = asset_server.load("ui/delete.png");

    // Outer container: fixed full-screen, clips vertically
    commands.spawn((
        ScreenRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(COLOR_BG),
    )).with_children(|outer| {
        // Fixed top bar: title
        outer.spawn(Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(16.0), Val::Px(8.0)),
            ..default()
        }).with_children(|bar| {
            spawn_title(bar, "My Parts & Builds");
        });

        // Scrollable middle area
        outer.spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: Val::Px(0.0),
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(20.0)),
                row_gap: Val::Px(8.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
        )).with_children(|root| {
            // Error banner (if a delete was blocked)
            if let Some(msg) = &error_msg {
                root.spawn((
                    Node {
                        padding: UiRect::all(Val::Px(10.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.8, 0.2, 0.2, 0.9)),
                )).with_children(|banner| {
                    banner.spawn((
                        Text::new(msg.clone()),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(COLOR_TEXT),
                    ));
                });
            }

            // ── Tops ──
            spawn_section_with_wheels(root, &registry.wheels, &asset_server, &edit_icon, &delete_icon);

            // ── Weapons ──
            spawn_section_with_parts(root, "Weapons", &registry.weapons, PartSlot::WeaponWheel, &asset_server, &edit_icon, &delete_icon);

            // ── Shafts ──
            spawn_section_with_shafts(root, &registry.shafts, &asset_server, &edit_icon, &delete_icon);

            // ── Chassis ──
            spawn_section_with_chassis(root, &registry.chassis, &asset_server, &edit_icon, &delete_icon);

            // ── Screws ──
            spawn_section_with_screws(root, &registry.screws, &asset_server, &edit_icon, &delete_icon);

            // ── Builds ──
            spawn_section_with_builds(root, &registry.builds, &edit_icon, &delete_icon);

            // Bottom padding so content doesn't sit against the button bar
            root.spawn(Node { height: Val::Px(8.0), ..default() });
        });

        // Fixed bottom bar: action buttons
        outer.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(16.0),
            padding: UiRect::all(Val::Px(12.0)),
            ..default()
        }).with_children(|row| {
            spawn_button(row, "New Build", ManageButton::NewBuild);
            spawn_button(row, "Back", ManageButton::Back);
        });
    });
}

fn spawn_section_with_parts(
    root: &mut ChildSpawnerCommands,
    title: &str,
    weapons: &std::collections::HashMap<String, WeaponWheelSpec>,
    _slot: PartSlot,
    asset_server: &AssetServer,
    edit_icon: &Handle<Image>,
    delete_icon: &Handle<Image>,
) {
    root.spawn((
        Text::new(title),
        TextFont { font_size: 18.0, ..default() },
        TextColor(COLOR_ACCENT),
        Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
    ));

    let mut ids: Vec<_> = weapons.keys().collect();
    ids.sort();

    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(8.0),
        justify_content: JustifyContent::Center,
        ..default()
    }).with_children(|grid| {
        for id in ids {
            let w = &weapons[id];
            let builtin = is_builtin(id);
            let img: Handle<Image> = asset_server.load(format!("weapons/{}.png", id));
            spawn_part_card(grid, id, &w.name, &format!("{:?}", w.kind), PartSlot::WeaponWheel, builtin, Some(img), edit_icon.clone(), delete_icon.clone());
        }
    });
}

fn spawn_section_with_shafts(
    root: &mut ChildSpawnerCommands,
    shafts: &std::collections::HashMap<String, ShaftSpec>,
    asset_server: &AssetServer,
    edit_icon: &Handle<Image>,
    delete_icon: &Handle<Image>,
) {
    root.spawn((
        Text::new("Shafts"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(COLOR_ACCENT),
        Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
    ));

    let mut ids: Vec<_> = shafts.keys().collect();
    ids.sort();

    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(8.0),
        justify_content: JustifyContent::Center,
        ..default()
    }).with_children(|grid| {
        for id in ids {
            let s = &shafts[id];
            let builtin = is_builtin(id);
            let img: Handle<Image> = asset_server.load(format!("shafts/{}.png", id));
            spawn_part_card(grid, id, &s.name, &format!("Stab:{:.1} Eff:{:.1}", s.stability, s.spin_efficiency), PartSlot::Shaft, builtin, Some(img), edit_icon.clone(), delete_icon.clone());
        }
    });
}

fn spawn_section_with_chassis(
    root: &mut ChildSpawnerCommands,
    chassis: &std::collections::HashMap<String, ChassisSpec>,
    asset_server: &AssetServer,
    edit_icon: &Handle<Image>,
    delete_icon: &Handle<Image>,
) {
    root.spawn((
        Text::new("Chassis"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(COLOR_ACCENT),
        Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
    ));

    let mut ids: Vec<_> = chassis.keys().collect();
    ids.sort();

    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(8.0),
        justify_content: JustifyContent::Center,
        ..default()
    }).with_children(|grid| {
        for id in ids {
            let c = &chassis[id];
            let builtin = is_builtin(id);
            let img: Handle<Image> = asset_server.load(format!("chassis/{}.png", id));
            spawn_part_card(grid, id, &c.name, &format!("Spd+{:.0}x{:.1}", c.move_speed_add, c.move_speed_mul), PartSlot::Chassis, builtin, Some(img), edit_icon.clone(), delete_icon.clone());
        }
    });
}

fn spawn_section_with_screws(
    root: &mut ChildSpawnerCommands,
    screws: &std::collections::HashMap<String, TraitScrewSpec>,
    asset_server: &AssetServer,
    edit_icon: &Handle<Image>,
    delete_icon: &Handle<Image>,
) {
    root.spawn((
        Text::new("Screws"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(COLOR_ACCENT),
        Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
    ));

    let mut ids: Vec<_> = screws.keys().collect();
    ids.sort();

    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(8.0),
        justify_content: JustifyContent::Center,
        ..default()
    }).with_children(|grid| {
        for id in ids {
            let s = &screws[id];
            let builtin = is_builtin(id);
            let img: Handle<Image> = asset_server.load(format!("screws/{}.png", id));
            spawn_part_card(grid, id, &s.name, &format!("HP+{:.0} CR:{:.1}", s.passive.spin_hp_max_add, s.passive.control_reduction), PartSlot::TraitScrew, builtin, Some(img), edit_icon.clone(), delete_icon.clone());
        }
    });
}

fn spawn_section_with_builds(
    root: &mut ChildSpawnerCommands,
    builds: &std::collections::HashMap<String, crate::game::parts::registry::BuildRef>,
    edit_icon: &Handle<Image>,
    delete_icon: &Handle<Image>,
) {
    root.spawn((
        Text::new("Builds"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(COLOR_ACCENT),
        Node { margin: UiRect::top(Val::Px(12.0)), ..default() },
    ));

    let mut ids: Vec<_> = builds.keys().collect();
    ids.sort();

    if ids.is_empty() {
        root.spawn((
            Text::new("(Use 'New Build' to assemble parts)"),
            TextFont { font_size: 13.0, ..default() },
            TextColor(COLOR_TEXT_DIM),
        ));
        return;
    }

    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(8.0),
        justify_content: JustifyContent::Center,
        ..default()
    }).with_children(|grid| {
        for id in ids {
            let b = &builds[id];
            let builtin = is_builtin(id);
            let stats = format!("{} + {}", b.wheel_id, b.weapon_id);
            let id_str: String = id.clone();
            let id_str2: String = id.clone();
            spawn_card_frame(grid, &b.name, &stats, None, COLOR_CARD, 220.0, move |card| {
                if !builtin {
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    }).with_children(|row| {
                        spawn_icon_button(row, edit_icon.clone(), ManageButton::EditBuild(id_str));
                        spawn_icon_button(row, delete_icon.clone(), ManageButton::DeleteBuild(id_str2));
                    });
                } else {
                    card.spawn((
                        Text::new("(built-in)"),
                        TextFont { font_size: 10.0, ..default() },
                        TextColor(COLOR_TEXT_DIM),
                    ));
                }
            });
        }
    });
}

fn spawn_section_with_wheels(
    root: &mut ChildSpawnerCommands,
    tops: &std::collections::HashMap<String, BaseStats>,
    asset_server: &AssetServer,
    edit_icon: &Handle<Image>,
    delete_icon: &Handle<Image>,
) {
    root.spawn((
        Text::new("Tops"),
        TextFont { font_size: 18.0, ..default() },
        TextColor(COLOR_ACCENT),
        Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
    ));

    let mut ids: Vec<_> = tops.keys().collect();
    ids.sort();

    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(8.0),
        justify_content: JustifyContent::Center,
        ..default()
    }).with_children(|grid| {
        for id in ids {
            let t = &tops[id];
            let builtin = is_builtin(id);
            let img: Handle<Image> = asset_server.load(format!("tops/{}.png", id));
            spawn_wheel_card(grid, id, &t.name, &format!("HP:{:.0} R:{:.2}", t.spin_hp_max.0, t.radius.0), builtin, Some(img), edit_icon.clone(), delete_icon.clone());
        }
    });
}

fn spawn_wheel_card(
    parent: &mut ChildSpawnerCommands,
    id: &str,
    name: &str,
    stats_line: &str,
    builtin: bool,
    image: Option<Handle<Image>>,
    edit_icon: Handle<Image>,
    delete_icon: Handle<Image>,
) {
    let id_str: String = id.into();
    let id_str2: String = id.into();
    spawn_card_frame(parent, name, stats_line, image, COLOR_CARD, 200.0, move |card| {
        if !builtin {
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            }).with_children(|row| {
                spawn_icon_button(row, edit_icon, ManageButton::EditWheel(id_str));
                spawn_icon_button(row, delete_icon, ManageButton::DeleteWheel(id_str2));
            });
        } else {
            card.spawn((
                Text::new("(built-in)"),
                TextFont { font_size: 10.0, ..default() },
                TextColor(COLOR_TEXT_DIM),
            ));
        }
    });
}

fn spawn_part_card(
    parent: &mut ChildSpawnerCommands,
    id: &str,
    name: &str,
    stats_line: &str,
    slot: PartSlot,
    builtin: bool,
    image: Option<Handle<Image>>,
    edit_icon: Handle<Image>,
    delete_icon: Handle<Image>,
) {
    let id_str: String = id.into();
    let id_str2: String = id.into();
    spawn_card_frame(parent, name, stats_line, image, COLOR_CARD, 200.0, move |card| {
        if !builtin {
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            }).with_children(|row| {
                spawn_icon_button(row, edit_icon, ManageButton::EditPart { slot, id: id_str });
                spawn_icon_button(row, delete_icon, ManageButton::DeletePart { slot, id: id_str2 });
            });
        } else {
            card.spawn((
                Text::new("(built-in)"),
                TextFont { font_size: 10.0, ..default() },
                TextColor(COLOR_TEXT_DIM),
            ));
        }
    });
}

fn spawn_icon_button<C: Component>(
    parent: &mut ChildSpawnerCommands,
    icon_handle: Handle<Image>,
    marker: C,
) {
    parent.spawn((
        marker,
        Button,
        Node {
            width: Val::Px(28.0),
            height: Val::Px(28.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
    )).with_children(|btn| {
        btn.spawn((
            ImageNode { image: icon_handle, ..default() },
            Node {
                width: Val::Px(22.0),
                height: Val::Px(22.0),
                ..default()
            },
        ));
    });
}

fn manage_parts_system(
    mut q: Query<(&Interaction, &ManageButton, &mut BackgroundColor), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut state: ResMut<DesignState>,
    mut registry: ResMut<PartRegistry>,
    repo: Option<Res<SqliteRepo>>,
    rt: Option<Res<TokioRuntime>>,
) {
    for (interaction, button, mut bg) in &mut q {
        if *interaction == Interaction::Pressed {
            match button {
                ManageButton::EditWheel(id) => {
                    state.editing_part_id = Some(id.clone());
                    state.return_to_manage = true;
                    next_state.set(GamePhase::EditWheel);
                }
                ManageButton::DeleteWheel(id) => {
                    let used_by = builds_using_part(&registry, id);
                    if !used_by.is_empty() {
                        state.delete_error = Some(format!(
                            "Cannot delete '{}': used by builds: {}", id, used_by.join(", ")
                        ));
                    } else {
                        if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                            let _ = repo.delete_part_sync(&rt.0, id);
                        }
                        let _ = std::fs::remove_file(format!("assets/tops/{}.png", id));
                        registry.wheels.remove(id.as_str());
                    }
                    next_state.set(GamePhase::ManageParts);
                }
                ManageButton::EditPart { slot, id } => {
                    state.editing_part_id = Some(id.clone());
                    state.return_to_manage = true;
                    match slot {
                        PartSlot::WeaponWheel => next_state.set(GamePhase::EditWeapon),
                        PartSlot::Shaft => next_state.set(GamePhase::EditShaft),
                        PartSlot::Chassis => next_state.set(GamePhase::EditChassis),
                        PartSlot::TraitScrew => next_state.set(GamePhase::EditScrew),
                    }
                }
                ManageButton::DeletePart { slot, id } => {
                    let used_by = builds_using_part(&registry, id);
                    if !used_by.is_empty() {
                        state.delete_error = Some(format!(
                            "Cannot delete '{}': used by builds: {}", id, used_by.join(", ")
                        ));
                    } else {
                        if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                            let _ = repo.delete_part_sync(&rt.0, id);
                        }
                        let dir = slot_dir(slot);
                        let _ = std::fs::remove_file(format!("assets/{}/{}.png", dir, id));
                        if *slot == PartSlot::WeaponWheel {
                            let _ = std::fs::remove_file(format!("assets/projectiles/{}_projectile.png", id));
                        }
                        match slot {
                            PartSlot::WeaponWheel => { registry.weapons.remove(id.as_str()); }
                            PartSlot::Shaft => { registry.shafts.remove(id.as_str()); }
                            PartSlot::Chassis => { registry.chassis.remove(id.as_str()); }
                            PartSlot::TraitScrew => { registry.screws.remove(id.as_str()); }
                        }
                    }
                    next_state.set(GamePhase::ManageParts);
                }
                ManageButton::EditBuild(id) => {
                    state.current_build_id = Some(id.clone());
                    next_state.set(GamePhase::AssembleBuild);
                }
                ManageButton::DeleteBuild(id) => {
                    if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                        let _ = repo.delete_build_sync(&rt.0, id);
                    }
                    registry.builds.remove(id);
                    next_state.set(GamePhase::ManageParts);
                }
                ManageButton::NewBuild => {
                    state.current_build_id = None;
                    state.current_build_wheel_id = "default_top".into();
                    state.current_build_weapon_id = "basic_blade".into();
                    state.current_build_shaft_id = "standard_shaft".into();
                    state.current_build_chassis_id = "standard_chassis".into();
                    state.current_build_screw_id = "standard_screw".into();
                    state.current_build_note.clear();
                    next_state.set(GamePhase::AssembleBuild);
                }
                ManageButton::Back => {
                    next_state.set(GamePhase::DesignHub);
                }
            }
        }
        // Icon buttons: subtle hover. Text buttons: standard hover.
        match button {
            ManageButton::EditWheel(_) | ManageButton::DeleteWheel(_) |
            ManageButton::EditPart { .. } | ManageButton::DeletePart { .. } |
            ManageButton::EditBuild(_) | ManageButton::DeleteBuild(_) => {
                match interaction {
                    Interaction::Hovered => *bg = BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                    Interaction::None => *bg = BackgroundColor(Color::NONE),
                    _ => {}
                }
            }
            _ => hover_system(interaction, &mut bg),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TOP EDITOR
// ═══════════════════════════════════════════════════════════════════════

#[derive(Component)]
enum EditorButton { Save, Cancel, SetImage }

fn spawn_wheel_editor(
    mut commands: Commands,
    state: Res<DesignState>,
    registry: Res<PartRegistry>,
    asset_server: Res<AssetServer>,
) {
    let t = state.editing_part_id.as_ref()
        .and_then(|id| registry.wheels.get(id))
        .cloned()
        .unwrap_or(BaseStats {
            id: String::new(),
            name: "My Top".into(),
            ..Default::default()
        });

    commands.spawn((
        ScreenRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(30.0)),
            row_gap: Val::Px(12.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        BackgroundColor(COLOR_BG),
    )).with_children(|root| {
        let title = if state.return_to_manage { "Edit Wheel" } else { "New Wheel" };
        spawn_title(root, title);

        let img = state.editing_part_id.as_ref().map(|id| asset_server.load(format!("tops/{}.png", id)));
        spawn_image_preview(root, img, 96.0);

        spawn_field_row(root, "Name", "Display name", "name", &t.name);
        spawn_field_row(root, "Max HP", "Max spin HP", "spin_hp_max", &format!("{}", t.spin_hp_max.0));
        spawn_field_row(root, "Radius", "Collision radius (world units)", "radius", &format!("{}", t.radius.0));
        spawn_field_row(root, "Move Speed", "Movement speed", "move_speed", &format!("{}", t.move_speed.0));
        spawn_field_row(root, "Accel", "Acceleration", "accel", &format!("{}", t.accel));
        spawn_field_row(root, "Control Reduction", "Control effect reduction (0.0=none)", "control_reduction", &format!("{}", t.control_reduction));

        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            margin: UiRect::top(Val::Px(16.0)),
            ..default()
        }).with_children(|row| {
            spawn_button(row, "Set Image", EditorButton::SetImage);
            spawn_button(row, "Save", EditorButton::Save);
            spawn_button(row, "Cancel", EditorButton::Cancel);
        });
    });
}

fn wheel_editor_system(
    mut q: Query<(&Interaction, &EditorButton, &mut BackgroundColor), Changed<Interaction>>,
    inputs: Query<&TextInput>,
    mut next_state: ResMut<NextState<GamePhase>>,
    state: ResMut<DesignState>,
    mut registry: ResMut<PartRegistry>,
    repo: Option<Res<SqliteRepo>>,
    rt: Option<Res<TokioRuntime>>,
) {
    for (interaction, button, mut bg) in &mut q {
        if *interaction == Interaction::Pressed {
            match button {
                EditorButton::Save => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    let name = read_field(&inputs, "name");
                    let spec = BaseStats {
                        id: id.clone(),
                        name: if name.is_empty() { "My Top".into() } else { name },
                        spin_hp_max: SpinHp(read_f32(&inputs, "spin_hp_max", 100.0)),
                        radius: Radius(read_f32(&inputs, "radius", 1.3)),
                        move_speed: MetersPerSec(read_f32(&inputs, "move_speed", 10.0)),
                        accel: read_f32(&inputs, "accel", 25.0),
                        control_reduction: read_f32(&inputs, "control_reduction", 0.0),
                        sprite_path: None,
                    };
                    if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                        let json = serde_json::to_string(&spec).unwrap_or_default();
                        let _ = repo.save_part_sync(&rt.0, "top", "top", &id, &json);
                    }
                    registry.wheels.insert(id, spec);
                    next_state.set(if state.return_to_manage { GamePhase::ManageParts } else { GamePhase::DesignHub });
                }
                EditorButton::Cancel => {
                    next_state.set(if state.return_to_manage { GamePhase::ManageParts } else { GamePhase::DesignHub });
                }
                EditorButton::SetImage => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    pick_and_copy_image("tops", &id);
                }
            }
        }
        hover_system(interaction, &mut bg);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SHAFT EDITOR
// ═══════════════════════════════════════════════════════════════════════

fn spawn_shaft_editor(
    mut commands: Commands,
    state: Res<DesignState>,
    registry: Res<PartRegistry>,
    asset_server: Res<AssetServer>,
) {
    let (name, stability, efficiency) = if let Some(id) = &state.editing_part_id {
        if let Some(s) = registry.shafts.get(id) {
            (s.name.clone(), s.stability, s.spin_efficiency)
        } else {
            ("My Shaft".into(), 0.5, 1.0)
        }
    } else {
        ("My Shaft".into(), 0.5, 1.0)
    };

    commands.spawn((
        ScreenRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(30.0)),
            row_gap: Val::Px(12.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        BackgroundColor(COLOR_BG),
    )).with_children(|root| {
        let title = if state.return_to_manage { "Edit Shaft" } else { "New Shaft" };
        spawn_title(root, title);

        // Image preview
        let img = state.editing_part_id.as_ref().map(|id| asset_server.load(format!("shafts/{}.png", id)));
        spawn_image_preview(root, img, 96.0);

        spawn_field_row(root, "Name", "Display name", "name", &name);
        spawn_field_row(root, "Stability", "Reduces knockback from collisions", "stability", &format!("{}", stability));
        spawn_field_row(root, "Spin Efficiency", "Spin consumption multiplier (1.0=standard)", "spin_efficiency", &format!("{}", efficiency));

        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            margin: UiRect::top(Val::Px(16.0)),
            ..default()
        }).with_children(|row| {
            spawn_button(row, "Set Image", EditorButton::SetImage);
            spawn_button(row, "Save", EditorButton::Save);
            spawn_button(row, "Cancel", EditorButton::Cancel);
        });
    });
}

fn shaft_editor_system(
    mut q: Query<(&Interaction, &EditorButton, &mut BackgroundColor), Changed<Interaction>>,
    inputs: Query<&TextInput>,
    mut next_state: ResMut<NextState<GamePhase>>,
    state: ResMut<DesignState>,
    mut registry: ResMut<PartRegistry>,
    repo: Option<Res<SqliteRepo>>,
    rt: Option<Res<TokioRuntime>>,
) {
    for (interaction, button, mut bg) in &mut q {
        if *interaction == Interaction::Pressed {
            match button {
                EditorButton::Save => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    let name = read_field(&inputs, "name");
                    let spec = ShaftSpec {
                        id: id.clone(),
                        name: if name.is_empty() { "My Shaft".into() } else { name },
                        stability: read_f32(&inputs, "stability", 0.5),
                        spin_efficiency: read_f32(&inputs, "spin_efficiency", 1.0),
                    };
                    if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                        let json = serde_json::to_string(&spec).unwrap_or_default();
                        let _ = repo.save_part_sync(&rt.0, "shaft", "shaft", &id, &json);
                    }
                    registry.shafts.insert(id, spec);
                    next_state.set(if state.return_to_manage { GamePhase::ManageParts } else { GamePhase::DesignHub });
                }
                EditorButton::Cancel => {
                    next_state.set(if state.return_to_manage { GamePhase::ManageParts } else { GamePhase::DesignHub });
                }
                EditorButton::SetImage => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    pick_and_copy_image("shafts", &id);
                }
            }
        }
        hover_system(interaction, &mut bg);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// CHASSIS EDITOR
// ═══════════════════════════════════════════════════════════════════════

fn spawn_chassis_editor(
    mut commands: Commands,
    state: Res<DesignState>,
    registry: Res<PartRegistry>,
    asset_server: Res<AssetServer>,
) {
    let c = state.editing_part_id.as_ref()
        .and_then(|id| registry.chassis.get(id))
        .cloned()
        .unwrap_or_default();

    commands.spawn((
        ScreenRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(30.0)),
            row_gap: Val::Px(12.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        BackgroundColor(COLOR_BG),
    )).with_children(|root| {
        let title = if state.return_to_manage { "Edit Chassis" } else { "New Chassis" };
        spawn_title(root, title);

        let img = state.editing_part_id.as_ref().map(|id| asset_server.load(format!("chassis/{}.png", id)));
        spawn_image_preview(root, img, 96.0);

        spawn_field_row(root, "Name", "Display name", "name", &c.name);
        spawn_field_row(root, "Move Speed Add", "Flat movement speed bonus", "move_speed_add", &format!("{}", c.move_speed_add));
        spawn_field_row(root, "Move Speed Mul", "Movement speed multiplier (1.0=unchanged)", "move_speed_mul", &format!("{}", c.move_speed_mul));
        spawn_field_row(root, "Accel Add", "Flat acceleration bonus", "accel_add", &format!("{}", c.accel_add));
        spawn_field_row(root, "Accel Mul", "Acceleration multiplier (1.0=unchanged)", "accel_mul", &format!("{}", c.accel_mul));
        spawn_field_row(root, "Radius Add", "Collision radius bonus", "radius_add", &format!("{}", c.radius_add));
        spawn_field_row(root, "Radius Mul", "Collision radius multiplier (1.0=unchanged)", "radius_mul", &format!("{}", c.radius_mul));

        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            margin: UiRect::top(Val::Px(16.0)),
            ..default()
        }).with_children(|row| {
            spawn_button(row, "Set Image", EditorButton::SetImage);
            spawn_button(row, "Save", EditorButton::Save);
            spawn_button(row, "Cancel", EditorButton::Cancel);
        });
    });
}

fn chassis_editor_system(
    mut q: Query<(&Interaction, &EditorButton, &mut BackgroundColor), Changed<Interaction>>,
    inputs: Query<&TextInput>,
    mut next_state: ResMut<NextState<GamePhase>>,
    state: ResMut<DesignState>,
    mut registry: ResMut<PartRegistry>,
    repo: Option<Res<SqliteRepo>>,
    rt: Option<Res<TokioRuntime>>,
) {
    for (interaction, button, mut bg) in &mut q {
        if *interaction == Interaction::Pressed {
            match button {
                EditorButton::Save => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    let name = read_field(&inputs, "name");
                    let spec = ChassisSpec {
                        id: id.clone(),
                        name: if name.is_empty() { "My Chassis".into() } else { name },
                        move_speed_add: read_f32(&inputs, "move_speed_add", 0.0),
                        move_speed_mul: read_f32(&inputs, "move_speed_mul", 1.0),
                        accel_add: read_f32(&inputs, "accel_add", 0.0),
                        accel_mul: read_f32(&inputs, "accel_mul", 1.0),
                        radius_add: read_f32(&inputs, "radius_add", 0.0),
                        radius_mul: read_f32(&inputs, "radius_mul", 1.0),
                    };
                    if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                        let json = serde_json::to_string(&spec).unwrap_or_default();
                        let _ = repo.save_part_sync(&rt.0, "chassis", "chassis", &id, &json);
                    }
                    registry.chassis.insert(id, spec);
                    next_state.set(if state.return_to_manage { GamePhase::ManageParts } else { GamePhase::DesignHub });
                }
                EditorButton::Cancel => {
                    next_state.set(if state.return_to_manage { GamePhase::ManageParts } else { GamePhase::DesignHub });
                }
                EditorButton::SetImage => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    pick_and_copy_image("chassis", &id);
                }
            }
        }
        hover_system(interaction, &mut bg);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SCREW EDITOR
// ═══════════════════════════════════════════════════════════════════════

fn spawn_screw_editor(
    mut commands: Commands,
    state: Res<DesignState>,
    registry: Res<PartRegistry>,
    asset_server: Res<AssetServer>,
) {
    let s = state.editing_part_id.as_ref()
        .and_then(|id| registry.screws.get(id))
        .cloned()
        .unwrap_or_default();

    commands.spawn((
        ScreenRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(30.0)),
            row_gap: Val::Px(12.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        BackgroundColor(COLOR_BG),
    )).with_children(|root| {
        let title = if state.return_to_manage { "Edit Screw" } else { "New Screw" };
        spawn_title(root, title);

        let img = state.editing_part_id.as_ref().map(|id| asset_server.load(format!("screws/{}.png", id)));
        spawn_image_preview(root, img, 96.0);

        spawn_field_row(root, "Name", "Display name", "name", &s.name);
        spawn_field_row(root, "Max HP Add", "Max spin (HP) bonus", "spin_hp_max_add", &format!("{}", s.passive.spin_hp_max_add));
        spawn_field_row(root, "Control Reduction", "Control effect reduction (stun/slow/knockback)", "control_reduction", &format!("{}", s.passive.control_reduction));
        spawn_field_row(root, "Damage Out Mul", "Outgoing damage multiplier (1.0=normal)", "damage_out_mult", &format!("{}", s.passive.damage_out_mult));
        spawn_field_row(root, "Damage In Mul", "Incoming damage multiplier (<1.0=tankier)", "damage_in_mult", &format!("{}", s.passive.damage_in_mult));

        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            margin: UiRect::top(Val::Px(16.0)),
            ..default()
        }).with_children(|row| {
            spawn_button(row, "Set Image", EditorButton::SetImage);
            spawn_button(row, "Save", EditorButton::Save);
            spawn_button(row, "Cancel", EditorButton::Cancel);
        });
    });
}

fn screw_editor_system(
    mut q: Query<(&Interaction, &EditorButton, &mut BackgroundColor), Changed<Interaction>>,
    inputs: Query<&TextInput>,
    mut next_state: ResMut<NextState<GamePhase>>,
    state: ResMut<DesignState>,
    mut registry: ResMut<PartRegistry>,
    repo: Option<Res<SqliteRepo>>,
    rt: Option<Res<TokioRuntime>>,
) {
    for (interaction, button, mut bg) in &mut q {
        if *interaction == Interaction::Pressed {
            match button {
                EditorButton::Save => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    let name = read_field(&inputs, "name");
                    let spec = TraitScrewSpec {
                        id: id.clone(),
                        name: if name.is_empty() { "My Screw".into() } else { name },
                        passive: crate::game::parts::trait_screw::TraitPassive {
                            spin_hp_max_add: read_f32(&inputs, "spin_hp_max_add", 0.0),
                            control_reduction: read_f32(&inputs, "control_reduction", 0.0),
                            damage_out_mult: read_f32(&inputs, "damage_out_mult", 1.0),
                            damage_in_mult: read_f32(&inputs, "damage_in_mult", 1.0),
                        },
                        hooks: vec![],
                    };
                    if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                        let json = serde_json::to_string(&spec).unwrap_or_default();
                        let _ = repo.save_part_sync(&rt.0, "screw", "screw", &id, &json);
                    }
                    registry.screws.insert(id, spec);
                    next_state.set(if state.return_to_manage { GamePhase::ManageParts } else { GamePhase::DesignHub });
                }
                EditorButton::Cancel => {
                    next_state.set(if state.return_to_manage { GamePhase::ManageParts } else { GamePhase::DesignHub });
                }
                EditorButton::SetImage => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    pick_and_copy_image("screws", &id);
                }
            }
        }
        hover_system(interaction, &mut bg);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WEAPON EDITOR
// ═══════════════════════════════════════════════════════════════════════

#[derive(Component)]
enum WeaponEditorButton { Save, Cancel, SetImage, SetProjectileImage }

#[derive(Component)]
struct KindSelector {
    current: WeaponKind,
    just_pressed: bool,
}

#[derive(Component)]
struct KindSelectorLabel;

#[derive(Component)]
struct MeleeSection;

#[derive(Component)]
struct RangedSection;

fn kind_display_text(kind: WeaponKind) -> &'static str {
    match kind {
        WeaponKind::Melee => "Melee",
        WeaponKind::Ranged => "Ranged",
    }
}

fn next_kind(kind: WeaponKind) -> WeaponKind {
    match kind {
        WeaponKind::Melee => WeaponKind::Ranged,
        WeaponKind::Ranged => WeaponKind::Melee,
    }
}

fn spawn_weapon_editor(
    mut commands: Commands,
    state: Res<DesignState>,
    registry: Res<PartRegistry>,
    asset_server: Res<AssetServer>,
) {
    let w = state.editing_part_id.as_ref()
        .and_then(|id| registry.weapons.get(id))
        .cloned()
        .unwrap_or(WeaponWheelSpec {
            id: String::new(),
            name: "My Weapon".into(),
            kind: WeaponKind::Melee,
            melee: Some(MeleeSpec::default()),
            ranged: None,
            sprite_path: None,
            projectile_sprite_path: None,
        });

    let kind = w.kind;
    let m = w.melee.unwrap_or_default();
    let r = w.ranged.unwrap_or_default();

    commands.spawn((
        ScreenRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(20.0)),
            row_gap: Val::Px(8.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        BackgroundColor(COLOR_BG),
    )).with_children(|root| {
        let title = if state.return_to_manage { "Edit Weapon" } else { "New Weapon" };
        spawn_title(root, title);

        let img = state.editing_part_id.as_ref().map(|id| asset_server.load(format!("weapons/{}.png", id)));
        spawn_image_preview(root, img, 96.0);

        spawn_field_row(root, "Name", "Display name", "name", &w.name);

        // Kind selector (cycling button)
        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        }).with_children(|row| {
            row.spawn(Node {
                width: Val::Px(200.0),
                flex_direction: FlexDirection::Column,
                ..default()
            }).with_children(|col| {
                col.spawn((
                    Text::new("Kind"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(COLOR_TEXT),
                ));
                col.spawn((
                    Text::new("Click to cycle weapon type"),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(COLOR_TEXT_DIM),
                ));
            });
            row.spawn((
                KindSelector { current: kind, just_pressed: false },
                Button,
                Node {
                    width: Val::Px(180.0),
                    height: Val::Px(32.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(COLOR_BTN),
            )).with_children(|btn| {
                btn.spawn((
                    KindSelectorLabel,
                    Text::new(kind_display_text(kind)),
                    TextFont { font_size: 15.0, ..default() },
                    TextColor(COLOR_ACCENT),
                ));
            });
        });

        let show_melee = kind == WeaponKind::Melee;

        // Melee section (shown when kind == Melee)
        root.spawn((
            MeleeSection,
            Node {
                display: if show_melee { Display::Flex } else { Display::None },
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
        )).with_children(|section| {
            section.spawn((
                Text::new("── Melee ──"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(COLOR_ACCENT),
            ));
            spawn_field_row(section, "Base Damage", "Base damage per hit", "m_base_damage", &format!("{}", m.base_damage));
            spawn_field_row(section, "Hit Cooldown", "Cooldown between hits on same target (sec)", "m_hit_cooldown", &format!("{}", m.hit_cooldown));
            spawn_field_row(section, "Hitbox Radius", "Attack hitbox distance", "m_hitbox_radius", &format!("{}", m.hitbox_radius));
            spawn_field_row(section, "Hitbox Angle", "Attack arc angle (radians)", "m_hitbox_angle", &format!("{}", m.hitbox_angle));
            spawn_field_row(section, "Blade Len", "Blade length (world units)", "m_blade_len", &format!("{}", m.blade_len));
            spawn_field_row(section, "Blade Thick", "Blade thickness", "m_blade_thick", &format!("{}", m.blade_thick));
            spawn_field_row(section, "Spin Rate Mul", "Visual spin rate multiplier", "m_spin_rate", &format!("{}", m.spin_rate_multiplier));
        });

        // Ranged section (shown when kind == Ranged)
        root.spawn((
            RangedSection,
            Node {
                display: if !show_melee { Display::Flex } else { Display::None },
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
        )).with_children(|section| {
            section.spawn((
                Text::new("── Ranged ──"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(COLOR_ACCENT),
            ));
            spawn_field_row(section, "Proj Damage", "Damage per projectile", "r_proj_damage", &format!("{}", r.projectile_damage));
            spawn_field_row(section, "Fire Rate", "Shots per second", "r_fire_rate", &format!("{}", r.fire_rate));
            spawn_field_row(section, "Burst Count", "Projectiles per burst", "r_burst_count", &format!("{}", r.burst_count));
            spawn_field_row(section, "Spread Angle", "Spread angle (radians)", "r_spread_angle", &format!("{}", r.spread_angle));
            spawn_field_row(section, "Proj Radius", "Projectile radius", "r_proj_radius", &format!("{}", r.projectile_radius));
            spawn_field_row(section, "Lifetime", "Projectile lifetime (sec)", "r_lifetime", &format!("{}", r.lifetime.0));
            spawn_field_row(section, "Proj Speed", "Projectile speed", "r_proj_speed", &format!("{}", r.projectile_speed));
            spawn_field_row(section, "Barrel Len", "Barrel length", "r_barrel_len", &format!("{}", r.barrel_len));
            spawn_field_row(section, "Barrel Thick", "Barrel thickness", "r_barrel_thick", &format!("{}", r.barrel_thick));
            spawn_field_row(section, "Spin Rate Mul", "Visual spin rate multiplier", "r_spin_rate", &format!("{}", r.spin_rate_multiplier));
        });

        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            margin: UiRect::top(Val::Px(12.0)),
            ..default()
        }).with_children(|row| {
            spawn_button(row, "Set Image", WeaponEditorButton::SetImage);
            spawn_button(row, "Set Proj Image", WeaponEditorButton::SetProjectileImage);
            spawn_button(row, "Save", WeaponEditorButton::Save);
            spawn_button(row, "Cancel", WeaponEditorButton::Cancel);
        });
    });
}

fn weapon_editor_system(
    mut q: Query<(&Interaction, &WeaponEditorButton, &mut BackgroundColor), (Changed<Interaction>, Without<KindSelector>)>,
    mut kind_q: Query<(&Interaction, &mut KindSelector, &mut BackgroundColor, &Children), Without<WeaponEditorButton>>,
    mut kind_labels: Query<&mut Text, With<KindSelectorLabel>>,
    mut melee_sections: Query<&mut Node, (With<MeleeSection>, Without<RangedSection>)>,
    mut ranged_sections: Query<&mut Node, (With<RangedSection>, Without<MeleeSection>)>,
    inputs: Query<&TextInput>,
    mut next_state: ResMut<NextState<GamePhase>>,
    state: ResMut<DesignState>,
    mut registry: ResMut<PartRegistry>,
    repo: Option<Res<SqliteRepo>>,
    rt: Option<Res<TokioRuntime>>,
) {
    // Handle kind cycling (gate on just_pressed to prevent multi-frame firing)
    for (interaction, mut selector, mut bg, children) in &mut kind_q {
        if *interaction == Interaction::Pressed && !selector.just_pressed {
            selector.just_pressed = true;
            selector.current = next_kind(selector.current);
            for child in children.iter() {
                if let Ok(mut text) = kind_labels.get_mut(child) {
                    **text = kind_display_text(selector.current).into();
                }
            }
            let show_melee = selector.current == WeaponKind::Melee;
            for mut node in &mut melee_sections {
                node.display = if show_melee { Display::Flex } else { Display::None };
            }
            for mut node in &mut ranged_sections {
                node.display = if !show_melee { Display::Flex } else { Display::None };
            }
        }
        if *interaction != Interaction::Pressed {
            selector.just_pressed = false;
        }
        match interaction {
            Interaction::Hovered => *bg = BackgroundColor(COLOR_BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(COLOR_BTN),
            _ => {}
        }
    }

    for (interaction, button, mut bg) in &mut q {
        if *interaction == Interaction::Pressed {
            match button {
                WeaponEditorButton::Save => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    let name = read_field(&inputs, "name");
                    let kind = kind_q.iter().next()
                        .map(|(_, s, _, _)| s.current)
                        .unwrap_or(WeaponKind::Melee);

                    let is_melee = kind == WeaponKind::Melee;
                    let is_ranged = kind == WeaponKind::Ranged;

                    let melee = if is_melee {
                        Some(MeleeSpec {
                            base_damage: read_f32(&inputs, "m_base_damage", 5.5),
                            hit_cooldown: read_f32(&inputs, "m_hit_cooldown", 0.5),
                            max_hits_per_rotation: 0,
                            hitbox_radius: read_f32(&inputs, "m_hitbox_radius", 2.5),
                            hitbox_angle: read_f32(&inputs, "m_hitbox_angle", 1.047),
                            hit_control: None,
                            spin_rate_multiplier: read_f32(&inputs, "m_spin_rate", 0.8),
                            blade_len: read_f32(&inputs, "m_blade_len", 2.3),
                            blade_thick: read_f32(&inputs, "m_blade_thick", 0.4),
                        })
                    } else { None };

                    let ranged = if is_ranged {
                        Some(RangedSpec {
                            projectile_damage: read_f32(&inputs, "r_proj_damage", 7.0),
                            fire_rate: read_f32(&inputs, "r_fire_rate", 3.0),
                            burst_count: read_u32(&inputs, "r_burst_count", 1),
                            spread_angle: read_f32(&inputs, "r_spread_angle", 0.0),
                            knockback_distance: 0.0,
                            projectile_radius: read_f32(&inputs, "r_proj_radius", 0.5),
                            control_duration: crate::game::stats::types::Seconds(0.0),
                            lifetime: crate::game::stats::types::Seconds(read_f32(&inputs, "r_lifetime", 2.0)),
                            projectile_speed: read_f32(&inputs, "r_proj_speed", 15.0),
                            aim_mode: crate::game::stats::types::AimMode::FollowSpin,
                            spin_rate_multiplier: read_f32(&inputs, "r_spin_rate", 0.3),
                            barrel_len: read_f32(&inputs, "r_barrel_len", 1.0),
                            barrel_thick: read_f32(&inputs, "r_barrel_thick", 0.3),
                        })
                    } else { None };

                    let spec = WeaponWheelSpec {
                        id: id.clone(),
                        name: if name.is_empty() { "My Weapon".into() } else { name },
                        kind,
                        melee,
                        ranged,
                        sprite_path: None,
                        projectile_sprite_path: None,
                    };
                    if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                        let json = serde_json::to_string(&spec).unwrap_or_default();
                        let _ = repo.save_part_sync(&rt.0, "weapon", &format!("{:?}", kind), &id, &json);
                    }
                    registry.weapons.insert(id, spec);
                    next_state.set(if state.return_to_manage { GamePhase::ManageParts } else { GamePhase::DesignHub });
                }
                WeaponEditorButton::Cancel => {
                    next_state.set(if state.return_to_manage { GamePhase::ManageParts } else { GamePhase::DesignHub });
                }
                WeaponEditorButton::SetImage => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    pick_and_copy_image("weapons", &id);
                }
                WeaponEditorButton::SetProjectileImage => {
                    let id = state.editing_part_id.clone().unwrap_or_else(gen_custom_id);
                    let dest = format!("assets/projectiles/{}_projectile.png", id);
                    if let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).pick_file() {
                        let _ = std::fs::create_dir_all("assets/projectiles");
                        let _ = std::fs::copy(&path, &dest);
                    }
                }
            }
        }
        hover_system(interaction, &mut bg);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ASSEMBLE BUILD
// ═══════════════════════════════════════════════════════════════════════

#[derive(Component)]
enum AssembleButton {
    ChangeTop,
    ChangeWeapon,
    ChangeShaft,
    ChangeChassis,
    ChangeScrew,
    SaveBuild,
    Back,
}

#[derive(Component)]
struct StatsPreviewText;

fn spawn_assemble_build(
    mut commands: Commands,
    state: Res<DesignState>,
    registry: Res<PartRegistry>,
    tuning: Res<Tuning>,
    asset_server: Res<AssetServer>,
) {
    let top_name = registry.wheels.get(&state.current_build_wheel_id).map(|t| t.name.as_str()).unwrap_or("?");
    let weapon_name = registry.weapons.get(&state.current_build_weapon_id).map(|w| w.name.as_str()).unwrap_or("?");
    let shaft_name = registry.shafts.get(&state.current_build_shaft_id).map(|s| s.name.as_str()).unwrap_or("?");
    let chassis_name = registry.chassis.get(&state.current_build_chassis_id).map(|c| c.name.as_str()).unwrap_or("?");
    let screw_name = registry.screws.get(&state.current_build_screw_id).map(|s| s.name.as_str()).unwrap_or("?");

    // Compute combined stats
    let stats_text = if let Some(build) = registry.resolve_build(
        "preview",
        "",
        &state.current_build_wheel_id,
        &state.current_build_weapon_id,
        &state.current_build_shaft_id,
        &state.current_build_chassis_id,
        &state.current_build_screw_id,
    ) {
        let mods = build.combined_modifiers();
        let eff = mods.compute_effective(&build.wheel, &tuning);
        format!(
            "HP: {:.0}  Radius: {:.2}  Speed: {:.1}\nAccel: {:.1}  Stab: {:.1}  Ctrl: {:.2}",
            eff.spin_hp_max.0, eff.radius.0, eff.move_speed.0,
            eff.accel, eff.stability, eff.control_multiplier
        )
    } else {
        "Invalid build (missing parts)".into()
    };

    commands.spawn((
        ScreenRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(20.0)),
            row_gap: Val::Px(10.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        BackgroundColor(COLOR_BG),
    )).with_children(|root| {
        spawn_title(root, "Assemble Build");

        spawn_field_row(root, "Build Name", "Optional note", "build_note", &state.current_build_note);

        // Slot cards
        let top_img: Handle<Image> = asset_server.load(format!("tops/{}.png", state.current_build_wheel_id));
        let wpn_img: Handle<Image> = asset_server.load(format!("weapons/{}.png", state.current_build_weapon_id));
        let shaft_img: Handle<Image> = asset_server.load(format!("shafts/{}.png", state.current_build_shaft_id));
        let chassis_img: Handle<Image> = asset_server.load(format!("chassis/{}.png", state.current_build_chassis_id));
        let screw_img: Handle<Image> = asset_server.load(format!("screws/{}.png", state.current_build_screw_id));

        spawn_slot_row(root, "Top Body", top_name, AssembleButton::ChangeTop, Some(top_img));
        spawn_slot_row(root, "Weapon", weapon_name, AssembleButton::ChangeWeapon, Some(wpn_img));
        spawn_slot_row(root, "Shaft", shaft_name, AssembleButton::ChangeShaft, Some(shaft_img));
        spawn_slot_row(root, "Chassis", chassis_name, AssembleButton::ChangeChassis, Some(chassis_img));
        spawn_slot_row(root, "Screw", screw_name, AssembleButton::ChangeScrew, Some(screw_img));

        // Stats preview
        root.spawn((
            Node {
                padding: UiRect::all(Val::Px(12.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(COLOR_CARD),
        )).with_children(|panel| {
            panel.spawn((
                StatsPreviewText,
                Text::new(stats_text),
                TextFont { font_size: 14.0, ..default() },
                TextColor(COLOR_TEXT),
            ));
        });

        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            margin: UiRect::top(Val::Px(12.0)),
            ..default()
        }).with_children(|row| {
            spawn_button(row, "Save Build", AssembleButton::SaveBuild);
            spawn_button(row, "Back", AssembleButton::Back);
        });
    });
}

fn spawn_slot_row<C: Component>(parent: &mut ChildSpawnerCommands, slot_label: &str, current_name: &str, change_button: C, image: Option<Handle<Image>>) {
    parent.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        ..default()
    }).with_children(|row| {
        spawn_image_preview(row, image, 32.0);
        row.spawn((
            Text::new(format!("{}: {}", slot_label, current_name)),
            TextFont { font_size: 16.0, ..default() },
            TextColor(COLOR_TEXT),
            Node { width: Val::Px(280.0), ..default() },
        ));
        spawn_button(row, "Change...", change_button);
    });
}

fn assemble_build_system(
    mut q: Query<(&Interaction, &AssembleButton, &mut BackgroundColor), Changed<Interaction>>,
    inputs: Query<&TextInput>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut state: ResMut<DesignState>,
    mut registry: ResMut<PartRegistry>,
    repo: Option<Res<SqliteRepo>>,
    rt: Option<Res<TokioRuntime>>,
) {
    for (interaction, button, mut bg) in &mut q {
        if *interaction == Interaction::Pressed {
            match button {
                AssembleButton::ChangeTop => {
                    state.picking_slot = Some(PartSlot::Shaft); // reuse for top body selection
                    // Actually use a special marker — we'll handle "top" in picker
                    state.picking_slot = None; // Special: None = top body
                    next_state.set(GamePhase::PickDesignPart);
                }
                AssembleButton::ChangeWeapon => {
                    state.picking_slot = Some(PartSlot::WeaponWheel);
                    next_state.set(GamePhase::PickDesignPart);
                }
                AssembleButton::ChangeShaft => {
                    state.picking_slot = Some(PartSlot::Shaft);
                    next_state.set(GamePhase::PickDesignPart);
                }
                AssembleButton::ChangeChassis => {
                    state.picking_slot = Some(PartSlot::Chassis);
                    next_state.set(GamePhase::PickDesignPart);
                }
                AssembleButton::ChangeScrew => {
                    state.picking_slot = Some(PartSlot::TraitScrew);
                    next_state.set(GamePhase::PickDesignPart);
                }
                AssembleButton::SaveBuild => {
                    let note = read_field(&inputs, "build_note");
                    state.current_build_note = note.clone();
                    let build_id = state.current_build_id.clone().unwrap_or_else(gen_custom_id);
                    let display_name = if note.is_empty() { build_id.clone() } else { note.clone() };

                    if let Some(build) = registry.resolve_build(
                        &build_id,
                        &display_name,
                        &state.current_build_wheel_id,
                        &state.current_build_weapon_id,
                        &state.current_build_shaft_id,
                        &state.current_build_chassis_id,
                        &state.current_build_screw_id,
                    ) {
                        let mut build = build;
                        build.note = if note.is_empty() { None } else { Some(note.clone()) };
                        if let (Some(repo), Some(rt)) = (repo.as_ref(), rt.as_ref()) {
                            let _ = repo.save_build_sync(&rt.0, &build);
                        }
                        // Register build in memory so it's available in the game picker
                        registry.builds.insert(build_id.clone(), crate::game::parts::registry::BuildRef {
                            id: build_id,
                            name: display_name,
                            wheel_id: state.current_build_wheel_id.clone(),
                            weapon_id: state.current_build_weapon_id.clone(),
                            shaft_id: state.current_build_shaft_id.clone(),
                            chassis_id: state.current_build_chassis_id.clone(),
                            screw_id: state.current_build_screw_id.clone(),
                        });
                    }
                    next_state.set(GamePhase::ManageParts);
                }
                AssembleButton::Back => {
                    next_state.set(GamePhase::ManageParts);
                }
            }
        }
        hover_system(interaction, &mut bg);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// PICK DESIGN PART
// ═══════════════════════════════════════════════════════════════════════

#[derive(Component)]
enum PickPartButton {
    Select(String),
    Back,
}

fn spawn_pick_design_part(
    mut commands: Commands,
    state: Res<DesignState>,
    registry: Res<PartRegistry>,
    asset_server: Res<AssetServer>,
) {
    let slot = &state.picking_slot;

    commands.spawn((
        ScreenRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(20.0)),
            row_gap: Val::Px(12.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        BackgroundColor(COLOR_BG),
    )).with_children(|root| {
        let title = match slot {
            None => "Select Top Body",
            Some(PartSlot::WeaponWheel) => "Select Weapon",
            Some(PartSlot::Shaft) => "Select Shaft",
            Some(PartSlot::Chassis) => "Select Chassis",
            Some(PartSlot::TraitScrew) => "Select Screw",
        };
        spawn_title(root, title);

        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(12.0),
            row_gap: Val::Px(12.0),
            justify_content: JustifyContent::Center,
            ..default()
        }).with_children(|grid| {
            match slot {
                None => {
                    let mut ids: Vec<_> = registry.wheels.keys().collect();
                    ids.sort();
                    for id in ids {
                        let t = &registry.wheels[id];
                        let img: Handle<Image> = asset_server.load(format!("tops/{}.png", id));
                        spawn_pick_card(grid, id, &t.name, &format!("HP:{:.0} R:{:.2}", t.spin_hp_max.0, t.radius.0), Some(img));
                    }
                }
                Some(PartSlot::WeaponWheel) => {
                    let mut ids: Vec<_> = registry.weapons.keys().collect();
                    ids.sort();
                    for id in ids {
                        let w = &registry.weapons[id];
                        let img: Handle<Image> = asset_server.load(format!("weapons/{}.png", id));
                        spawn_pick_card(grid, id, &w.name, &format!("{:?}", w.kind), Some(img));
                    }
                }
                Some(PartSlot::Shaft) => {
                    let mut ids: Vec<_> = registry.shafts.keys().collect();
                    ids.sort();
                    for id in ids {
                        let s = &registry.shafts[id];
                        let img: Handle<Image> = asset_server.load(format!("shafts/{}.png", id));
                        spawn_pick_card(grid, id, &s.name, &format!("Stab:{:.1}", s.stability), Some(img));
                    }
                }
                Some(PartSlot::Chassis) => {
                    let mut ids: Vec<_> = registry.chassis.keys().collect();
                    ids.sort();
                    for id in ids {
                        let c = &registry.chassis[id];
                        let img: Handle<Image> = asset_server.load(format!("chassis/{}.png", id));
                        spawn_pick_card(grid, id, &c.name, &format!("Spd+{:.0}", c.move_speed_add), Some(img));
                    }
                }
                Some(PartSlot::TraitScrew) => {
                    let mut ids: Vec<_> = registry.screws.keys().collect();
                    ids.sort();
                    for id in ids {
                        let s = &registry.screws[id];
                        let img: Handle<Image> = asset_server.load(format!("screws/{}.png", id));
                        spawn_pick_card(grid, id, &s.name, &format!("HP+{:.0}", s.passive.spin_hp_max_add), Some(img));
                    }
                }
            }
        });

        root.spawn(Node { margin: UiRect::top(Val::Px(12.0)), ..default() }).with_children(|row| {
            spawn_button(row, "Back", PickPartButton::Back);
        });
    });
}

fn spawn_pick_card(parent: &mut ChildSpawnerCommands, id: &str, name: &str, stats: &str, image: Option<Handle<Image>>) {
    parent.spawn((
        PickPartButton::Select(id.into()),
        Button,
        Node {
            width: Val::Px(200.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(10.0)),
            row_gap: Val::Px(6.0),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(COLOR_CARD),
    )).with_children(|card| {
        spawn_image_preview(card, image, 64.0);
        card.spawn((
            Text::new(name),
            TextFont { font_size: 16.0, ..default() },
            TextColor(COLOR_TEXT),
        ));
        card.spawn((
            Text::new(stats),
            TextFont { font_size: 12.0, ..default() },
            TextColor(COLOR_TEXT_DIM),
        ));
    });
}

fn pick_design_part_system(
    mut q: Query<(&Interaction, &PickPartButton, &mut BackgroundColor), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut state: ResMut<DesignState>,
) {
    for (interaction, button, mut bg) in &mut q {
        if *interaction == Interaction::Pressed {
            match button {
                PickPartButton::Select(id) => {
                    match &state.picking_slot {
                        None => state.current_build_wheel_id = id.clone(),
                        Some(PartSlot::WeaponWheel) => state.current_build_weapon_id = id.clone(),
                        Some(PartSlot::Shaft) => state.current_build_shaft_id = id.clone(),
                        Some(PartSlot::Chassis) => state.current_build_chassis_id = id.clone(),
                        Some(PartSlot::TraitScrew) => state.current_build_screw_id = id.clone(),
                    }
                    next_state.set(GamePhase::AssembleBuild);
                }
                PickPartButton::Back => {
                    next_state.set(GamePhase::AssembleBuild);
                }
            }
        }
        match interaction {
            Interaction::Hovered => *bg = BackgroundColor(COLOR_CARD_SELECTED),
            Interaction::None => *bg = BackgroundColor(COLOR_CARD),
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// IMAGE HANDLING (rfd file dialog)
// ═══════════════════════════════════════════════════════════════════════

fn pick_and_copy_image(slot_dir: &str, part_id: &str) {
    let dest = format!("assets/{}/{}.png", slot_dir, part_id);
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("PNG Image", &["png"])
        .pick_file()
    {
        let _ = std::fs::create_dir_all(format!("assets/{}", slot_dir));
        let _ = std::fs::copy(&path, &dest);
    }
}
