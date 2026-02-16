use bevy::prelude::*;
use bevy::camera::ScalingMode;
use std::f32::consts::PI;

use crate::config::tuning::Tuning;
use crate::game::{
    arena::{circle, obstacle},
    collision, combat,
    components::*,
    events::{CollisionMessage, GameEvent},
    hooks,
    parts::registry::PartRegistry,
    physics,
    stats::types::*,
};
use crate::plugins::menu_plugin::{GameMode, GameSelection};

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
            (circle::despawn_projectiles_outside_arena, obstacle::cleanup_ttl, obstacle::handle_despawn_events)
                .chain()
                .in_set(FixedGameSet::CleanupSet),
        );

        // ── Startup: camera + registry (persist forever) ─────────────
        app.add_systems(Startup, setup_camera);

        // ── OnEnter(Aiming): spawn arena + tops from selection ───────
        app.add_systems(OnEnter(GamePhase::Aiming), setup_arena);

        // ── Aiming phase (Update) ───────────────────────────────────────
        app.add_systems(
            Update,
            (read_aim_input, read_aim_input_p2, ai_auto_aim, check_all_confirmed, update_aim_arrow)
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

        // ── Cleanup on return to MainMenu ────────────────────────────
        app.add_systems(OnEnter(GamePhase::MainMenu), cleanup_game);

        // ── Always-on ───────────────────────────────────────────────────
        app.add_systems(Update, tuning_reload_input);
    }
}

// ── Startup: camera + registry ───────────────────────────────────────

fn setup_camera(
    mut commands: Commands,
    tuning: Res<Tuning>,
) {
    let ppu = tuning.pixels_per_unit.max(1.0);

    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::WindowSize,
            scale: 1.0 / ppu,
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Part registry (persists forever)
    commands.insert_resource(PartRegistry::with_defaults());
}

// ── OnEnter(Aiming): spawn arena + tops ──────────────────────────────

fn setup_arena(
    mut commands: Commands,
    tuning: Res<Tuning>,
    registry: Res<PartRegistry>,
    selection: Res<GameSelection>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let ppu = tuning.pixels_per_unit.max(1.0);

    // Arena boundary
    let arena_mesh = meshes.add(Circle::new(tuning.arena_radius));
    commands.spawn((
        InGame,
        Mesh2d(arena_mesh),
        MeshMaterial2d(materials.add(Color::srgba(0.15, 0.15, 0.2, 1.0))),
        Transform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
    ));

    // Projectile assets
    let proj_mesh = meshes.add(Circle::new(1.0));
    let proj_mat = materials.add(Color::srgb(1.0, 1.0, 0.2));
    commands.insert_resource(ProjectileAssets {
        mesh: proj_mesh,
        material: proj_mat,
    });

    // ── Player 1 ─────────────────────────────────────────────────────
    let p1_build = registry
        .resolve_build(
            "p1_build",
            &selection.p1_top_id,
            &selection.p1_weapon_id,
            "standard_shaft",
            "standard_chassis",
            "standard_screw",
        )
        .expect("P1 build parts not found in registry");
    let p1_mods = p1_build.combined_modifiers();
    let p1_effective = p1_mods.compute_effective(&p1_build.top, &tuning);
    let p1_radius = p1_effective.radius.0;
    let p1_mesh = meshes.add(Circle::new(p1_radius));
    let p1_weapon_mesh = spawn_weapon_visual_mesh(&p1_build.weapon, p1_radius, &mut meshes);

    commands
        .spawn((
            InGame,
            Top,
            PlayerControlled,
            Mesh2d(p1_mesh),
            MeshMaterial2d(materials.add(Color::srgb(0.2, 0.6, 1.0))),
            Transform::from_translation(Vec3::new(-3.0, 0.0, 0.0)),
            Velocity(Vec2::ZERO),
            RotationAngle(AngleRad::new(0.0)),
            SpinHpCurrent(p1_effective.spin_hp_max),
            TopEffectiveStats(p1_effective.clone()),
            TopBuild(p1_build),
            ControlState::default(),
            StatusEffects::default(),
            (LaunchAim::default(), MeleeHitTracker::default(), combat::RangedFireTimer::default()),
        ))
        .with_children(|parent| {
            parent.spawn((
                WeaponVisual,
                Mesh2d(p1_weapon_mesh.0),
                MeshMaterial2d(materials.add(Color::srgb(0.9, 0.9, 1.0))),
                p1_weapon_mesh.1,
            ));
        });

    // P1 aim arrow
    let arrow_len = tuning.aim_arrow_len_px / ppu;
    let arrow_thick = tuning.aim_arrow_thickness_px / ppu;
    let arrow_mesh = meshes.add(Rectangle::new(arrow_len, arrow_thick));
    commands.spawn((
        InGame,
        AimArrow,
        Mesh2d(arrow_mesh),
        MeshMaterial2d(materials.add(Color::srgb(0.2, 1.0, 0.2))),
        Transform::from_translation(Vec3::new(-3.0 + arrow_len * 0.5, 0.0, 1.0)),
    ));

    // ── Player 2 / AI ────────────────────────────────────────────────
    let p2_build = registry
        .resolve_build(
            "p2_build",
            &selection.p2_top_id,
            &selection.p2_weapon_id,
            "standard_shaft",
            "standard_chassis",
            "standard_screw",
        )
        .expect("P2 build parts not found in registry");
    let p2_mods = p2_build.combined_modifiers();
    let p2_effective = p2_mods.compute_effective(&p2_build.top, &tuning);
    let p2_radius = p2_effective.radius.0;
    let p2_mesh = meshes.add(Circle::new(p2_radius));
    let p2_weapon_mesh = spawn_weapon_visual_mesh(&p2_build.weapon, p2_radius, &mut meshes);

    let mut p2_entity = commands.spawn((
        InGame,
        Top,
        Mesh2d(p2_mesh),
        MeshMaterial2d(materials.add(Color::srgb(1.0, 0.2, 0.2))),
        Transform::from_translation(Vec3::new(3.0, 0.0, 0.0)),
        Velocity(Vec2::ZERO),
        RotationAngle(AngleRad::new(PI)),
        SpinHpCurrent(p2_effective.spin_hp_max),
        TopEffectiveStats(p2_effective),
        TopBuild(p2_build),
        ControlState::default(),
        StatusEffects::default(),
        (LaunchAim { angle: PI, confirmed: false }, MeleeHitTracker::default(), combat::RangedFireTimer::default()),
    ));

    match selection.mode {
        GameMode::PvAI => { p2_entity.insert(AiControlled); }
        GameMode::PvP => { p2_entity.insert(Player2Controlled); }
    }

    p2_entity.with_children(|parent| {
        parent.spawn((
            WeaponVisual,
            Mesh2d(p2_weapon_mesh.0),
            MeshMaterial2d(materials.add(Color::srgb(1.0, 0.9, 0.8))),
            p2_weapon_mesh.1,
        ));
    });

    // P2 aim arrow (PvP only — AI auto-aims so no arrow needed)
    if selection.mode == GameMode::PvP {
        let arrow_mesh2 = meshes.add(Rectangle::new(arrow_len, arrow_thick));
        // P2 faces left (angle=PI), so offset arrow to the left
        let p2_dir = Vec2::new(PI.cos(), PI.sin());
        let p2_arrow_center = Vec2::new(3.0, 0.0) + p2_dir * (arrow_len * 0.5);
        commands.spawn((
            InGame,
            AimArrow,
            Player2Controlled,
            Mesh2d(arrow_mesh2),
            MeshMaterial2d(materials.add(Color::srgb(1.0, 0.4, 0.2))),
            Transform::from_translation(Vec3::new(p2_arrow_center.x, p2_arrow_center.y, 1.0))
                .with_rotation(Quat::from_rotation_z(PI)),
        ));
    }
}

/// Create weapon visual mesh + transform based on weapon spec.
fn spawn_weapon_visual_mesh(
    weapon: &crate::game::parts::weapon_wheel::WeaponWheelSpec,
    top_radius: f32,
    meshes: &mut ResMut<Assets<Mesh>>,
) -> (Handle<Mesh>, Transform) {
    let (len, thick) = match weapon.kind {
        WeaponKind::Ranged => {
            let r = weapon.ranged.as_ref().expect("Ranged weapon missing RangedSpec");
            (r.barrel_len, r.barrel_thick)
        }
        WeaponKind::Melee | WeaponKind::Hybrid => {
            let m = weapon.melee.as_ref().expect("Melee weapon missing MeleeSpec");
            (m.blade_len, m.blade_thick)
        }
    };
    let mesh = meshes.add(Rectangle::new(len, thick));
    let tf = Transform::from_translation(Vec3::new(top_radius + len * 0.5, 0.0, 0.5));
    (mesh, tf)
}

// ── Cleanup on return to MainMenu ────────────────────────────────────

fn cleanup_game(
    mut commands: Commands,
    query: Query<Entity, With<InGame>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<ProjectileAssets>();
}

// ── Aiming phase systems ────────────────────────────────────────────

/// Player 1 rotates with Arrow keys, confirms with Space.
fn read_aim_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    tuning: Res<Tuning>,
    mut query: Query<&mut LaunchAim, With<PlayerControlled>>,
) {
    let aim_speed = tuning.aim_speed;
    for mut aim in &mut query {
        if aim.confirmed {
            continue;
        }
        if keyboard.pressed(KeyCode::ArrowLeft) {
            aim.angle += aim_speed * time.delta_secs();
        }
        if keyboard.pressed(KeyCode::ArrowRight) {
            aim.angle -= aim_speed * time.delta_secs();
        }
        if keyboard.just_pressed(KeyCode::Space) {
            aim.confirmed = true;
        }
    }
}

/// Player 2 (PvP) rotates with A/D, confirms with Enter.
fn read_aim_input_p2(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    tuning: Res<Tuning>,
    mut query: Query<&mut LaunchAim, With<Player2Controlled>>,
) {
    let aim_speed = tuning.aim_speed;
    for mut aim in &mut query {
        if aim.confirmed {
            continue;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            aim.angle += aim_speed * time.delta_secs();
        }
        if keyboard.pressed(KeyCode::KeyD) {
            aim.angle -= aim_speed * time.delta_secs();
        }
        if keyboard.just_pressed(KeyCode::Enter) {
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

/// Visual: position and rotate aim arrows to match aim direction.
fn update_aim_arrow(
    tuning: Res<Tuning>,
    player: Query<(&Transform, &LaunchAim), (With<PlayerControlled>, Without<AimArrow>)>,
    p2_top: Query<(&Transform, &LaunchAim), (With<Player2Controlled>, With<Top>, Without<AimArrow>)>,
    mut arrows_p1: Query<
        &mut Transform,
        (With<AimArrow>, Without<PlayerControlled>, Without<Player2Controlled>),
    >,
    mut arrows_p2: Query<
        &mut Transform,
        (With<AimArrow>, With<Player2Controlled>, Without<PlayerControlled>),
    >,
) {
    let ppu = tuning.pixels_per_unit.max(1.0);
    let arrow_offset = tuning.aim_arrow_offset_px / ppu;

    // P1 arrow
    if let Some((top_tf, aim)) = player.iter().next() {
        let top_pos = top_tf.translation.truncate();
        let dir = Vec2::new(aim.angle.cos(), aim.angle.sin());
        let arrow_center = top_pos + dir * arrow_offset;
        for mut arrow_tf in &mut arrows_p1 {
            arrow_tf.translation = Vec3::new(arrow_center.x, arrow_center.y, 1.0);
            arrow_tf.rotation = Quat::from_rotation_z(aim.angle);
        }
    }

    // P2 arrow (PvP only)
    if let Some((top_tf, aim)) = p2_top.iter().next() {
        let top_pos = top_tf.translation.truncate();
        let dir = Vec2::new(aim.angle.cos(), aim.angle.sin());
        let arrow_center = top_pos + dir * arrow_offset;
        for mut arrow_tf in &mut arrows_p2 {
            arrow_tf.translation = Vec3::new(arrow_center.x, arrow_center.y, 1.0);
            arrow_tf.rotation = Quat::from_rotation_z(aim.angle);
        }
    }
}

// ── OnEnter(Battle) systems ─────────────────────────────────────────

/// Set each top's velocity from its aim direction * move_speed.
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
