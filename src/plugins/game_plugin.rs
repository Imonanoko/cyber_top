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
    let ppu = tuning.pixels_per_unit.max(1.0);

    // Camera (Bevy 0.18): 用 Camera2d + Projection::Orthographic
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::WindowSize,
            // scale 越小 = 看起來越放大
            // 目標：1 world unit 顯示成 20 px -> scale = 1/20
            scale: 1.0 / ppu,
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Arena boundary (filled circle) — 用 world units，不乘 ppu
    let arena_mesh = meshes.add(Circle::new(tuning.arena_radius));
    commands.spawn((
        Mesh2d(arena_mesh),
        MeshMaterial2d(materials.add(Color::srgba(0.15, 0.15, 0.2, 1.0))),
        Transform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
    ));

    // Projectile assets (unit circle scaled per-projectile)
    let proj_mesh = meshes.add(Circle::new(1.0));
    let proj_mat = materials.add(Color::srgb(1.0, 1.0, 0.2));
    commands.insert_resource(ProjectileAssets {
        mesh: proj_mesh,
        material: proj_mat,
    });

    // Part registry (future: load from DB)
    let registry = PartRegistry::with_defaults();

    // Player build: ranged weapon (looked up from registry)
    let player_build = registry
        .resolve_build(
            "player_build",
            "default_top",
            "basic_blaster",
            "standard_shaft",
            "standard_chassis",
            "standard_screw",
        )
        .expect("player build parts not found in registry");
    let player_mods = player_build.combined_modifiers();
    let player_effective = player_mods.compute_effective(&player_build.top, &tuning);

    let player_radius = player_effective.radius.0;
    let player_mesh = meshes.add(Circle::new(player_radius));

    // Weapon visual mesh based on weapon kind
    let player_weapon_mesh = spawn_weapon_visual_mesh(
        &player_build.weapon,
        player_radius,
        &mut meshes,
    );

    commands
        .spawn((
            Top,
            PlayerControlled,
            Mesh2d(player_mesh),
            MeshMaterial2d(materials.add(Color::srgb(0.2, 0.6, 1.0))),
            Transform::from_translation(Vec3::new(-3.0, 0.0, 0.0)),
            Velocity(Vec2::ZERO),
            RotationAngle(AngleRad::new(0.0)),
            SpinHpCurrent(player_effective.spin_hp_max),
            TopEffectiveStats(player_effective.clone()),
            TopBuild(player_build),
            ControlState::default(),
            StatusEffects::default(),
            LaunchAim::default(),
            MeleeHitTracker::default(),
            combat::RangedFireTimer::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                WeaponVisual,
                Mesh2d(player_weapon_mesh.0),
                MeshMaterial2d(materials.add(Color::srgb(0.9, 0.9, 1.0))),
                player_weapon_mesh.1,
            ));
        });

    let arrow_len = tuning.aim_arrow_len_px / ppu;
    let arrow_thick = tuning.aim_arrow_thickness_px / ppu;

    let arrow_mesh = meshes.add(Rectangle::new(arrow_len, arrow_thick));
    commands.spawn((
        AimArrow,
        Mesh2d(arrow_mesh),
        MeshMaterial2d(materials.add(Color::srgb(0.2, 1.0, 0.2))),
        Transform::from_translation(Vec3::new(-3.0 + arrow_len * 0.5, 0.0, 1.0)),
    ));

    // AI top: melee weapon (looked up from registry)
    let ai_build = registry
        .resolve_build(
            "ai_build",
            "default_top",
            "basic_blade",
            "standard_shaft",
            "standard_chassis",
            "standard_screw",
        )
        .expect("AI build parts not found in registry");
    let ai_mods = ai_build.combined_modifiers();
    let ai_effective = ai_mods.compute_effective(&ai_build.top, &tuning);
    let ai_radius = ai_effective.radius.0;
    let ai_mesh = meshes.add(Circle::new(ai_radius));

    let ai_weapon_mesh =
        spawn_weapon_visual_mesh(&ai_build.weapon, ai_radius, &mut meshes);

    commands
        .spawn((
            Top,
            AiControlled,
            Mesh2d(ai_mesh),
            MeshMaterial2d(materials.add(Color::srgb(1.0, 0.2, 0.2))),
            Transform::from_translation(Vec3::new(3.0, 0.0, 0.0)),
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
        ))
        .with_children(|parent| {
            parent.spawn((
                WeaponVisual,
                Mesh2d(ai_weapon_mesh.0),
                MeshMaterial2d(materials.add(Color::srgb(1.0, 0.9, 0.8))),
                ai_weapon_mesh.1,
            ));
        });

    // Insert registry as resource (for future runtime access)
    commands.insert_resource(registry);
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

// ── Aiming phase systems ────────────────────────────────────────────

/// Player rotates launch direction with Left/Right (or A/D), confirms with Space/Enter.
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
    tuning: Res<Tuning>,
    player: Query<(&Transform, &LaunchAim), With<PlayerControlled>>,
    mut arrows: Query<&mut Transform, (With<AimArrow>, Without<PlayerControlled>)>,
) {
    let Some((top_tf, aim)) = player.iter().next() else { return; };

    let ppu = tuning.pixels_per_unit.max(1.0);
    let arrow_offset = tuning.aim_arrow_offset_px / ppu;

    let top_pos = top_tf.translation.truncate();
    let dir = Vec2::new(aim.angle.cos(), aim.angle.sin());
    let arrow_center = top_pos + dir * arrow_offset;

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
