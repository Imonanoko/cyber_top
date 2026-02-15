use bevy::prelude::*;

use std::f32::consts::PI;

use crate::config::tuning::Tuning;
use crate::game::{
    arena::{circle, obstacle},
    collision, combat,
    components::*,
    events::{CollisionMessage, GameEvent},
    hooks,
    parts::Build,
    physics,
    stats::{base::BaseStats, types::*},
};

// ── SystemSets (strict FixedUpdate ordering, battle-phase only) ─────

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FixedGameSet {
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
        app.add_message::<GameEvent>();
        app.add_message::<CollisionMessage>();
        app.init_state::<GamePhase>();

        // Configure FixedUpdate set ordering (each set gated to Battle phase)
        app.configure_sets(
            FixedUpdate,
            (
                FixedGameSet::PhysicsSet
                    .run_if(in_state(GamePhase::Battle)),
                FixedGameSet::CollisionDetectSet
                    .run_if(in_state(GamePhase::Battle)),
                FixedGameSet::EventGenerateSet
                    .run_if(in_state(GamePhase::Battle)),
                FixedGameSet::HookProcessSet
                    .run_if(in_state(GamePhase::Battle)),
                FixedGameSet::EventApplySet
                    .run_if(in_state(GamePhase::Battle)),
                FixedGameSet::CleanupSet
                    .run_if(in_state(GamePhase::Battle)),
            )
                .chain(),
        );

        // PhysicsSet — chained to fix B0002 (parallel Transform/Velocity conflicts)
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
                .chain()
                .in_set(FixedGameSet::PhysicsSet),
        );

        // CollisionDetectSet
        app.add_systems(
            FixedUpdate,
            collision::detect_collisions.in_set(FixedGameSet::CollisionDetectSet),
        );

        // EventGenerateSet — chained to fix B0002 (MessageWriter conflicts)
        app.add_systems(
            FixedUpdate,
            (
                combat::generate_collision_damage,
                combat::detect_melee_hits,
                combat::fire_ranged_weapons,
            )
                .chain()
                .in_set(FixedGameSet::EventGenerateSet),
        );

        // HookProcessSet
        app.add_systems(
            FixedUpdate,
            hooks::process_hooks.in_set(FixedGameSet::HookProcessSet),
        );

        // EventApplySet — chained to fix B0002
        app.add_systems(
            FixedUpdate,
            (
                combat::apply_damage_events,
                combat::apply_control_events,
                combat::resolve_top_collisions,
                obstacle::spawn_obstacles,
                obstacle::spawn_projectiles,
            )
                .chain()
                .in_set(FixedGameSet::EventApplySet),
        );

        // CleanupSet
        app.add_systems(
            FixedUpdate,
            (obstacle::cleanup_ttl, obstacle::handle_despawn_events)
                .chain()
                .in_set(FixedGameSet::CleanupSet),
        );

        // ── Startup ─────────────────────────────────────────────────────
        app.add_systems(Startup, setup_game);

        // ── Aiming phase (Update) ───────────────────────────────────────
        app.add_systems(
            Update,
            (read_aim_input, ai_auto_aim, check_all_confirmed, update_aim_arrow)
                .chain()
                .run_if(in_state(GamePhase::Aiming)),
        );

        // ── OnEnter(Battle): launch tops + despawn aim arrows ───────────
        app.add_systems(
            OnEnter(GamePhase::Battle),
            (launch_tops, despawn_aim_arrows),
        );

        // ── Battle → GameOver check ─────────────────────────────────────
        app.add_systems(
            Update,
            check_game_over.run_if(in_state(GamePhase::Battle)),
        );

        // ── Always-on ───────────────────────────────────────────────────
        app.add_systems(Update, tuning_reload_input);
    }
}

// ── Startup ─────────────────────────────────────────────────────────

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
        LaunchAim::default(),
        MeleeHitTracker::default(),
        combat::RangedFireTimer::default(),
    ));

    // Aim arrow for the player top
    let arrow_mesh = meshes.add(Rectangle::new(60.0, 4.0));
    commands.spawn((
        AimArrow,
        Mesh2d(arrow_mesh),
        MeshMaterial2d(materials.add(Color::srgb(0.2, 1.0, 0.2))),
        Transform::from_translation(Vec3::new(-3.0 * scale + 30.0, 0.0, 1.0)),
    ));

    // AI top
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
        RotationAngle(AngleRad::new(PI)),
        SpinHpCurrent(ai_effective.spin_hp_max),
        TopEffectiveStats(ai_effective),
        TopBuild(ai_build),
        ControlState::default(),
        StatusEffects::default(),
        LaunchAim::default(),
        MeleeHitTracker::default(),
        combat::RangedFireTimer::default(),
    ));
}

// ── Aiming phase systems ────────────────────────────────────────────

/// Player rotates launch direction with Left/Right (or A/D), confirms with Space/Enter.
fn read_aim_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut LaunchAim, With<PlayerControlled>>,
) {
    let aim_speed = 3.0; // radians per second
    for mut aim in &mut query {
        if aim.confirmed {
            continue;
        }
        if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
            aim.angle += aim_speed * time.delta_secs();
        }
        if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
            aim.angle -= aim_speed * time.delta_secs();
        }
        if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::Enter) {
            aim.confirmed = true;
        }
    }
}

/// AI auto-aims with a pseudo-random direction and confirms immediately.
fn ai_auto_aim(mut query: Query<&mut LaunchAim, With<AiControlled>>) {
    for mut aim in &mut query {
        if !aim.confirmed {
            aim.angle = pseudo_random_angle();
            aim.confirmed = true;
        }
    }
}

fn pseudo_random_angle() -> f32 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos as f32 / 1_000_000_000.0) * 2.0 * PI
}

/// When all tops have confirmed their aim, transition to Battle.
fn check_all_confirmed(
    query: Query<&LaunchAim, With<Top>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    if query.iter().all(|aim| aim.confirmed) {
        next_state.set(GamePhase::Battle);
    }
}

/// Visual: position and rotate the aim arrow to match the player's aim direction.
fn update_aim_arrow(
    player: Query<(&Transform, &LaunchAim), With<PlayerControlled>>,
    mut arrows: Query<&mut Transform, (With<AimArrow>, Without<PlayerControlled>)>,
) {
    let Some((top_tf, aim)) = player.iter().next() else {
        return;
    };
    let top_pos = top_tf.translation.truncate();
    let dir = Vec2::new(aim.angle.cos(), aim.angle.sin());
    let arrow_center = top_pos + dir * 40.0;

    for mut arrow_tf in &mut arrows {
        arrow_tf.translation = Vec3::new(arrow_center.x, arrow_center.y, 1.0);
        arrow_tf.rotation = Quat::from_rotation_z(aim.angle);
    }
}

// ── OnEnter(Battle) systems ─────────────────────────────────────────

/// Set each top's velocity from its aim direction × move_speed.
fn launch_tops(
    mut query: Query<(&LaunchAim, &mut Velocity, &TopEffectiveStats), With<Top>>,
) {
    for (aim, mut vel, stats) in &mut query {
        let dir = Vec2::new(aim.angle.cos(), aim.angle.sin());
        vel.0 = dir * stats.0.move_speed.0;
    }
}

/// Despawn aim arrow entities.
fn despawn_aim_arrows(mut commands: Commands, arrows: Query<Entity, With<AimArrow>>) {
    for entity in &arrows {
        commands.entity(entity).despawn();
    }
}

// ── Battle phase systems ────────────────────────────────────────────

/// Transition to GameOver when any top's spin HP reaches 0.
fn check_game_over(
    query: Query<&SpinHpCurrent, With<Top>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    for spin in &query {
        if spin.0 .0 <= 0.0 {
            next_state.set(GamePhase::GameOver);
            return;
        }
    }
}

// ── Always-on ───────────────────────────────────────────────────────

/// Reload tuning with F5.
fn tuning_reload_input(keyboard: Res<ButtonInput<KeyCode>>, mut tuning: ResMut<Tuning>) {
    if keyboard.just_pressed(KeyCode::F5) {
        tuning.reload();
    }
}
