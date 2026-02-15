use bevy::prelude::*;

use crate::config::tuning::Tuning;
use crate::game::{
    arena::{circle, obstacle},
    collision, combat,
    components::*,
    events::GameEvent,
    hooks,
    intent::Intent,
    parts::Build,
    physics,
    stats::{base::BaseStats, types::*},
};

// ── SystemSets (strict FixedUpdate ordering) ────────────────────────

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FixedGameSet {
    InputIntentSet,
    PhysicsSet,
    CollisionDetectSet,
    EventGenerateSet,
    HookProcessSet,
    EventApplySet,
    CleanupSet,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        // Register message type (Bevy 0.18: Message replaces Event)
        app.add_message::<GameEvent>();

        // Configure FixedUpdate set ordering
        app.configure_sets(
            FixedUpdate,
            (
                FixedGameSet::InputIntentSet,
                FixedGameSet::PhysicsSet,
                FixedGameSet::CollisionDetectSet,
                FixedGameSet::EventGenerateSet,
                FixedGameSet::HookProcessSet,
                FixedGameSet::EventApplySet,
                FixedGameSet::CleanupSet,
            )
                .chain(),
        );

        // InputIntentSet
        app.add_systems(
            FixedUpdate,
            physics::apply_intent.in_set(FixedGameSet::InputIntentSet),
        );

        // PhysicsSet
        app.add_systems(
            FixedUpdate,
            (
                physics::integrate_physics,
                physics::integrate_projectiles,
                physics::spin_drain,
                physics::tick_control_state,
                physics::tick_status_effects,
                physics::tick_melee_trackers,
                circle::wall_reflection,
            )
                .in_set(FixedGameSet::PhysicsSet),
        );

        // CollisionDetectSet
        app.add_systems(
            FixedUpdate,
            collision::detect_collisions.in_set(FixedGameSet::CollisionDetectSet),
        );

        // EventGenerateSet
        app.add_systems(
            FixedUpdate,
            (
                combat::generate_collision_damage,
                combat::detect_melee_hits,
                combat::fire_ranged_weapons,
            )
                .in_set(FixedGameSet::EventGenerateSet),
        );

        // HookProcessSet
        app.add_systems(
            FixedUpdate,
            hooks::process_hooks.in_set(FixedGameSet::HookProcessSet),
        );

        // EventApplySet
        app.add_systems(
            FixedUpdate,
            (
                combat::apply_damage_events,
                combat::apply_control_events,
                combat::resolve_top_collisions,
                obstacle::spawn_obstacles,
                obstacle::spawn_projectiles,
            )
                .in_set(FixedGameSet::EventApplySet),
        );

        // CleanupSet
        app.add_systems(
            FixedUpdate,
            (obstacle::cleanup_ttl, obstacle::handle_despawn_events)
                .in_set(FixedGameSet::CleanupSet),
        );

        // Startup: spawn arena and initial tops
        app.add_systems(Startup, setup_game);

        // Update: read input
        app.add_systems(Update, (read_player_input, tuning_reload_input));
    }
}

/// Setup: spawn player top, AI top, and arena visualization.
fn setup_game(
    mut commands: Commands,
    tuning: Res<Tuning>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Camera
    commands.spawn(Camera2d);

    let scale = 20.0; // pixels per world unit

    // Arena boundary (filled circle)
    let arena_mesh = meshes.add(Circle::new(tuning.arena_radius * scale));
    commands.spawn((
        Mesh2d(arena_mesh),
        MeshMaterial2d(materials.add(Color::srgba(0.15, 0.15, 0.2, 1.0))),
        Transform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
    ));

    let default_build = Build::default();
    let base = BaseStats::default();
    let mods = default_build.combined_modifiers();
    let effective = mods.compute_effective(&base, &tuning);

    // Player top
    let player_radius = effective.radius.0;
    let player_mesh = meshes.add(Circle::new(player_radius * scale));
    commands.spawn((
        Top,
        PlayerControlled,
        Mesh2d(player_mesh),
        MeshMaterial2d(materials.add(Color::srgb(0.2, 0.6, 1.0))),
        Transform::from_translation(Vec3::new(-3.0 * scale, 0.0, 0.0)),
        Velocity(Vec2::ZERO),
        RotationAngle(AngleRad::new(0.0)),
        SpinHpCurrent(effective.spin_hp_max),
        TopEffectiveStats(effective.clone()),
        TopBuild(default_build.clone()),
        ControlState::default(),
        StatusEffects::default(),
        Intent::default(),
        MeleeHitTracker::default(),
        combat::RangedFireTimer::default(),
    ));

    // AI top (simple opponent)
    let ai_build = Build {
        id: "ai_build".into(),
        top_id: "ai_top".into(),
        ..Default::default()
    };
    let ai_mods = ai_build.combined_modifiers();
    let ai_effective = ai_mods.compute_effective(&base, &tuning);
    let ai_radius = ai_effective.radius.0;
    let ai_mesh = meshes.add(Circle::new(ai_radius * scale));

    commands.spawn((
        Top,
        AiControlled,
        Mesh2d(ai_mesh),
        MeshMaterial2d(materials.add(Color::srgb(1.0, 0.2, 0.2))),
        Transform::from_translation(Vec3::new(3.0 * scale, 0.0, 0.0)),
        Velocity(Vec2::ZERO),
        RotationAngle(AngleRad::new(std::f32::consts::PI)),
        SpinHpCurrent(ai_effective.spin_hp_max),
        TopEffectiveStats(ai_effective),
        TopBuild(ai_build),
        ControlState::default(),
        StatusEffects::default(),
        Intent::default(),
        MeleeHitTracker::default(),
        combat::RangedFireTimer::default(),
    ));
}

/// Read keyboard input and write to Intent component.
fn read_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Intent, With<PlayerControlled>>,
) {
    for mut intent in &mut query {
        let mut dir = Vec2::ZERO;
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            dir.y += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            dir.y -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        intent.move_dir = dir.normalize_or_zero();
        intent.fire = keyboard.pressed(KeyCode::Space);
    }
}

/// Reload tuning with F5.
fn tuning_reload_input(keyboard: Res<ButtonInput<KeyCode>>, mut tuning: ResMut<Tuning>) {
    if keyboard.just_pressed(KeyCode::F5) {
        tuning.reload();
    }
}
