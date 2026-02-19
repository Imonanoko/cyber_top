use bevy::prelude::*;
use bevy::camera::ScalingMode;
use std::collections::HashMap;
use std::f32::consts::PI;

use crate::assets_map::GameAssets;
use crate::assets_map::SfxHandles;
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
        // Zone systems (speed/damage/gravity) run BEFORE integrate_physics
        // so their effects are applied in the same frame (no deferred Commands).
        app.add_systems(
            FixedUpdate,
            (
                speed_boost_system,
                speed_boost_tick,
                damage_boost_system,
                gravity_device_system,
                physics::integrate_physics,
                physics::integrate_projectiles,
                physics::spin_drain,
                physics::tick_control_state,
                physics::tick_melee_trackers,
                circle::wall_reflection,
                obstacle::static_obstacle_bounce,
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
                obstacle::spawn_projectiles,
            )
                .chain()
                .in_set(FixedGameSet::EventApplySet),
        );

        // CleanupSet
        app.add_systems(
            FixedUpdate,
            (circle::despawn_projectiles_outside_arena, obstacle::cleanup_ttl, obstacle::handle_despawn_events, play_sound_effects)
                .chain()
                .in_set(FixedGameSet::CleanupSet),
        );

        // ── Startup: camera + registry + assets ──────────────────────
        app.add_systems(Startup, (setup_camera, load_game_assets).chain());

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
    repo: Option<Res<crate::storage::sqlite_repo::SqliteRepo>>,
    tokio_rt: Option<Res<crate::plugins::storage_plugin::TokioRuntime>>,
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

    // Part registry: hardcoded defaults + custom parts/builds from DB
    let mut registry = PartRegistry::with_defaults();
    if let (Some(repo), Some(rt)) = (repo, tokio_rt) {
        registry.merge_custom_parts(&repo, &rt.0);
        registry.merge_custom_builds(&repo, &rt.0);
        registry.merge_custom_maps(&repo, &rt.0);
    }
    commands.insert_resource(registry);
}

// ── Startup: load all game assets ────────────────────────────────────

fn load_game_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    registry: Res<PartRegistry>,
) {
    let mut top_sprites = HashMap::new();
    let mut weapon_sprites = HashMap::new();
    let mut projectile_sprites = HashMap::new();
    let mut fallback_colors = HashMap::new();

    // Load top sprites
    for (id, stats) in &registry.tops {
        let path = stats.sprite_path.clone()
            .unwrap_or_else(|| format!("tops/{}.png", id));
        top_sprites.insert(id.clone(), asset_server.load(&path));
    }

    // Load weapon sprites
    for (id, weapon) in &registry.weapons {
        let path = weapon.sprite_path.clone()
            .unwrap_or_else(|| format!("weapons/{}.png", id));
        weapon_sprites.insert(id.clone(), asset_server.load(&path));

        // Load projectile sprite for ranged weapons
        if weapon.ranged.is_some() {
            let proj_path = weapon.projectile_sprite_path.clone()
                .unwrap_or_else(|| format!("projectiles/{}_projectile.png", id));
            projectile_sprites.insert(id.clone(), asset_server.load(&proj_path));
        }
    }

    // Fallback colors (used when sprite files are missing)
    fallback_colors.insert("default_top".into(), Color::srgb(0.2, 0.6, 1.0));
    fallback_colors.insert("basic_blade".into(), Color::srgb(0.9, 0.9, 1.0));
    fallback_colors.insert("basic_blaster".into(), Color::srgb(0.9, 0.9, 1.0));

    // Load SFX
    let sfx = SfxHandles {
        launch: asset_server.load("audio/sfx/launch.ogg"),
        collision_top: asset_server.load("audio/sfx/collision_top.ogg"),
        collision_wall: asset_server.load("audio/sfx/collision_wall.ogg"),
        melee_hit: asset_server.load("audio/sfx/melee_hit.ogg"),
        ranged_fire: asset_server.load("audio/sfx/ranged_fire.ogg"),
        projectile_hit: asset_server.load("audio/sfx/projectile_hit.ogg"),
    };

    commands.insert_resource(GameAssets {
        top_sprites,
        weapon_sprites,
        projectile_sprites,
        fallback_colors,
        sfx,
    });
}

// ── Visual helpers ───────────────────────────────────────────────────

/// Insert top visual components: sprite if available, else procedural mesh.
fn insert_top_visual(
    entity: &mut EntityCommands,
    top_id: &str,
    radius: f32,
    game_assets: &GameAssets,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) {
    if let Some(sprite_handle) = game_assets.top_sprite(top_id) {
        let diameter = radius * 2.0;
        entity.insert(Sprite {
            image: sprite_handle.clone(),
            custom_size: Some(Vec2::new(diameter, diameter)),
            ..default()
        });
    } else {
        let mesh = meshes.add(Circle::new(radius));
        let color = game_assets.fallback_color(top_id);
        entity.insert((
            Mesh2d(mesh),
            MeshMaterial2d(materials.add(color)),
        ));
    }
}

/// Spawn weapon visual child entity: sprite if available, else procedural mesh.
fn spawn_weapon_visual(
    parent: &mut ChildSpawnerCommands,
    weapon: &crate::game::parts::weapon_wheel::WeaponWheelSpec,
    top_radius: f32,
    game_assets: &GameAssets,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) {
    let (len, thick) = match weapon.kind {
        WeaponKind::Ranged => {
            let r = weapon.ranged.as_ref().expect("Ranged weapon missing RangedSpec");
            (r.barrel_len, r.barrel_thick)
        }
        WeaponKind::Melee => {
            let m = weapon.melee.as_ref().expect("Melee weapon missing MeleeSpec");
            (m.blade_len, m.blade_thick)
        }
    };
    let tf = Transform::from_translation(Vec3::new(top_radius + len * 0.5, 0.0, 0.5));

    if let Some(sprite_handle) = game_assets.weapon_sprite(&weapon.id) {
        parent.spawn((
            WeaponVisual,
            Sprite {
                image: sprite_handle.clone(),
                custom_size: Some(Vec2::new(len, thick)),
                ..default()
            },
            tf,
        ));
    } else {
        let mesh = meshes.add(Rectangle::new(len, thick));
        let color = game_assets.fallback_color(&weapon.id);
        parent.spawn((
            WeaponVisual,
            Mesh2d(mesh),
            MeshMaterial2d(materials.add(color)),
            tf,
        ));
    }
}

// ── OnEnter(Aiming): spawn arena + tops ──────────────────────────────

fn setup_arena(
    mut commands: Commands,
    tuning: Res<Tuning>,
    registry: Res<PartRegistry>,
    selection: Res<GameSelection>,
    game_assets: Res<GameAssets>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let ppu = tuning.pixels_per_unit.max(1.0);

    // Look up map from registry
    let map_spec = registry.maps.get(&selection.map_id);
    let arena_radius = map_spec.map(|m| m.arena_radius).unwrap_or(tuning.arena_radius);

    // Arena boundary
    let arena_mesh = meshes.add(Circle::new(arena_radius));
    commands.spawn((
        InGame,
        Mesh2d(arena_mesh),
        MeshMaterial2d(materials.add(Color::srgba(0.15, 0.15, 0.2, 1.0))),
        Transform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
    ));

    // Store the actual arena radius for use by physics systems
    commands.insert_resource(ArenaRadius(arena_radius));

    // Spawn map placements
    if let Some(map) = map_spec {
        let mut obs_count = 0u32;
        let mut gravity_count = 0u32;
        let mut speed_count = 0u32;
        let mut damage_count = 0u32;

        for placement in &map.placements {
            let wx = placement.grid_x as f32 * crate::game::map::GRID_CELL_SIZE;
            let wy = placement.grid_y as f32 * crate::game::map::GRID_CELL_SIZE;
            let pos = Vec3::new(wx, wy, 0.0);
            let cell_radius = crate::game::map::GRID_CELL_SIZE * 0.5;

            match placement.item {
                crate::game::map::MapItem::Obstacle => {
                    obs_count += 1;
                    commands.spawn((
                        InGame,
                        StaticObstacle,
                        ObstacleMarker,
                        CollisionRadius(cell_radius),
                        ObstacleBehavior(CollisionBehavior::DamageOnHit),
                        ObstacleOwner,
                        Sprite {
                            image: asset_server.load("obstacles/obstacle.png"),
                            custom_size: Some(Vec2::splat(crate::game::map::GRID_CELL_SIZE)),
                            ..default()
                        },
                        Transform::from_translation(pos),
                    ));
                }
                crate::game::map::MapItem::GravityDevice => {
                    gravity_count += 1;
                    // Effect radius 3.0; visual circle sized to match
                    let effect_radius = 3.0_f32;
                    commands.spawn((
                        InGame,
                        GravityDevice {
                            radius: effect_radius,
                        },
                        CollisionRadius(cell_radius),
                        Sprite {
                            image: asset_server.load("obstacles/gravity_device.png"),
                            custom_size: Some(Vec2::splat(effect_radius * 2.0)),
                            ..default()
                        },
                        Transform::from_translation(pos),
                    ));
                }
                crate::game::map::MapItem::SpeedBoost => {
                    speed_count += 1;
                    // Detection radius = half a grid cell; place 2×2 in editor for area coverage
                    commands.spawn((
                        InGame,
                        SpeedBoostZone {
                            multiplier: 1.5,
                            duration: 3.0,
                        },
                        CollisionRadius(cell_radius),
                        Sprite {
                            image: asset_server.load("obstacles/speed_boost.png"),
                            custom_size: Some(Vec2::splat(crate::game::map::GRID_CELL_SIZE)),
                            ..default()
                        },
                        Transform::from_translation(pos.with_z(-0.5)),
                    ));
                }
                crate::game::map::MapItem::DamageBoost => {
                    damage_count += 1;
                    commands.spawn((
                        InGame,
                        DamageBoostZone { multiplier: 1.5 },
                        CollisionRadius(cell_radius),
                        Sprite {
                            image: asset_server.load("obstacles/damage_boost.png"),
                            custom_size: Some(Vec2::splat(crate::game::map::GRID_CELL_SIZE)),
                            ..default()
                        },
                        Transform::from_translation(pos.with_z(-0.5)),
                    ));
                }
            }
        }
        info!(
            "Map '{}' loaded: {} obstacles, {} gravity, {} speed-boost, {} damage-boost zones",
            selection.map_id, obs_count, gravity_count, speed_count, damage_count
        );
    } else {
        info!("Map '{}' not found in registry — using default arena (no placements)", selection.map_id);
    }

    // Projectile assets (mesh fallback + sprite handles)
    let proj_mesh = meshes.add(Circle::new(1.0));
    let proj_mat = materials.add(Color::srgb(1.0, 1.0, 0.2));
    commands.insert_resource(ProjectileAssets {
        mesh: proj_mesh,
        material: proj_mat,
        sprites: game_assets.projectile_sprites.clone(),
    });

    // ── Player 1 ─────────────────────────────────────────────────────
    let p1_ref = registry.builds.get(&selection.p1_build_id)
        .expect("P1 build not found in registry");
    let p1_build = registry
        .resolve_build(
            &p1_ref.id,
            &p1_ref.top_id,
            &p1_ref.weapon_id,
            &p1_ref.shaft_id,
            &p1_ref.chassis_id,
            &p1_ref.screw_id,
        )
        .expect("P1 build parts not found in registry");
    let p1_top_id = p1_ref.top_id.clone();
    let p1_mods = p1_build.combined_modifiers();
    let p1_effective = p1_mods.compute_effective(&p1_build.top, &tuning);
    let p1_radius = p1_effective.radius.0;

    let mut p1_entity = commands.spawn((
        InGame,
        Top,
        PlayerControlled,
        Transform::from_translation(Vec3::new(-3.0, 0.0, 0.0)),
        Velocity(Vec2::ZERO),
        RotationAngle(AngleRad::new(0.0)),
        SpinHpCurrent(p1_effective.spin_hp_max),
        TopEffectiveStats(p1_effective.clone()),
        TopBuild(p1_build.clone()),
        ControlState::default(),
        (LaunchAim::default(), MeleeHitTracker::default(), combat::RangedFireTimer::default()),
        SpeedBoostEffect { expires_at: 0.0, multiplier: 1.0 },
        DamageBoostActive { multiplier: 1.0 },
    ));
    insert_top_visual(&mut p1_entity, &p1_top_id, p1_radius, &game_assets, &mut meshes, &mut materials);
    p1_entity.with_children(|parent| {
        spawn_weapon_visual(parent, &p1_build.weapon, p1_radius, &game_assets, &mut meshes, &mut materials);
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
    let p2_ref = registry.builds.get(&selection.p2_build_id)
        .expect("P2 build not found in registry");
    let p2_build = registry
        .resolve_build(
            &p2_ref.id,
            &p2_ref.top_id,
            &p2_ref.weapon_id,
            &p2_ref.shaft_id,
            &p2_ref.chassis_id,
            &p2_ref.screw_id,
        )
        .expect("P2 build parts not found in registry");
    let p2_top_id = p2_ref.top_id.clone();
    let p2_mods = p2_build.combined_modifiers();
    let p2_effective = p2_mods.compute_effective(&p2_build.top, &tuning);
    let p2_radius = p2_effective.radius.0;

    let mut p2_entity = commands.spawn((
        InGame,
        Top,
        Transform::from_translation(Vec3::new(3.0, 0.0, 0.0)),
        Velocity(Vec2::ZERO),
        RotationAngle(AngleRad::new(PI)),
        SpinHpCurrent(p2_effective.spin_hp_max),
        TopEffectiveStats(p2_effective),
        TopBuild(p2_build.clone()),
        ControlState::default(),
        (LaunchAim { angle: PI, confirmed: false }, MeleeHitTracker::default(), combat::RangedFireTimer::default()),
        SpeedBoostEffect { expires_at: 0.0, multiplier: 1.0 },
        DamageBoostActive { multiplier: 1.0 },
    ));

    match selection.mode {
        GameMode::PvAI => { p2_entity.insert(AiControlled); }
        GameMode::PvP => { p2_entity.insert(Player2Controlled); }
    }

    insert_top_visual(&mut p2_entity, &p2_top_id, p2_radius, &game_assets, &mut meshes, &mut materials);
    p2_entity.with_children(|parent| {
        spawn_weapon_visual(parent, &p2_build.weapon, p2_radius, &game_assets, &mut meshes, &mut materials);
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

// ── Cleanup on return to MainMenu ────────────────────────────────────

fn cleanup_game(
    mut commands: Commands,
    query: Query<Entity, With<InGame>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<ProjectileAssets>();
    commands.remove_resource::<ArenaRadius>();
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

/// Set each top's velocity from its aim direction * move_speed. Play launch sound.
fn launch_tops(
    mut commands: Commands,
    mut query: Query<(&LaunchAim, &mut Velocity, &TopEffectiveStats), With<Top>>,
    game_assets: Res<GameAssets>,
) {
    let mut launched = false;
    for (aim, mut vel, stats) in &mut query {
        let dir = Vec2::new(aim.angle.cos(), aim.angle.sin());
        vel.0 = dir * stats.0.move_speed.0;
        launched = true;
    }
    if launched {
        commands.spawn((
            AudioPlayer::<AudioSource>(game_assets.sfx.launch.clone()),
            PlaybackSettings::DESPAWN,
        ));
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

// ── Audio system ────────────────────────────────────────────────────

/// Play sound effects in response to game events (runs in CleanupSet).
fn play_sound_effects(
    mut commands: Commands,
    mut game_events: MessageReader<GameEvent>,
    mut collision_events: MessageReader<CollisionMessage>,
    game_assets: Res<GameAssets>,
) {
    // Top-top collision
    for _event in collision_events.read() {
        commands.spawn((
            AudioPlayer::<AudioSource>(game_assets.sfx.collision_top.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }

    for event in game_events.read() {
        match event {
            GameEvent::DealDamage { kind, .. } => {
                let handle = match kind {
                    DamageKind::Wall => Some(&game_assets.sfx.collision_wall),
                    DamageKind::Melee => Some(&game_assets.sfx.melee_hit),
                    DamageKind::Projectile => Some(&game_assets.sfx.projectile_hit),
                    _ => None,
                };
                if let Some(h) = handle {
                    commands.spawn((
                        AudioPlayer::<AudioSource>(h.clone()),
                        PlaybackSettings::DESPAWN,
                    ));
                }
            }
            GameEvent::SpawnProjectile { .. } => {
                commands.spawn((
                    AudioPlayer::<AudioSource>(game_assets.sfx.ranged_fire.clone()),
                    PlaybackSettings::DESPAWN,
                ));
            }
            _ => {}
        }
    }
}

// ── Map item battle systems ─────────────────────────────────────────

/// Gravity device: continuously steers tops toward the device while in range.
/// Each tick, blends velocity direction toward the device by `steer_strength * dt`.
fn gravity_device_system(
    tuning: Res<Tuning>,
    devices: Query<(&Transform, &GravityDevice)>,
    mut tops: Query<(&Transform, &mut Velocity, &TopEffectiveStats), (With<Top>, Without<GravityDevice>)>,
) {
    let dt = tuning.dt;
    // Steer strength: fraction of direction blended per second (higher = stronger pull)
    let steer_strength = 3.0_f32;

    for (dev_tf, device) in &devices {
        let dev_pos = dev_tf.translation.truncate();

        for (top_tf, mut vel, top_stats) in &mut tops {
            let top_pos = top_tf.translation.truncate();
            let top_radius = top_stats.0.radius.0;
            let dist = top_pos.distance(dev_pos);

            if dist < device.radius + top_radius && dist > 0.01 {
                let speed = vel.0.length();
                if speed > 0.01 {
                    let toward_device = (dev_pos - top_pos) / dist;
                    // Blend current direction toward device direction
                    let blend = (steer_strength * dt).min(1.0);
                    let current_dir = vel.0 / speed;
                    let new_dir = (current_dir * (1.0 - blend) + toward_device * blend).normalize();
                    vel.0 = new_dir * speed;
                }
            }
        }
    }
}

/// Speed boost: tops overlapping a SpeedBoostZone get a speed multiplier.
/// Mutates the always-present SpeedBoostEffect directly (no deferred Commands).
fn speed_boost_system(
    time: Res<Time>,
    zones: Query<(&Transform, &CollisionRadius, &SpeedBoostZone)>,
    mut tops: Query<(&Transform, &TopEffectiveStats, &mut SpeedBoostEffect), With<Top>>,
) {
    let now = time.elapsed_secs_f64();

    for (top_tf, top_stats, mut effect) in &mut tops {
        let top_pos = top_tf.translation.truncate();
        let top_radius = top_stats.0.radius.0;
        let mut in_zone = false;
        let mut best_mult = 1.0_f32;
        let mut best_dur = 0.0_f32;

        for (zone_tf, zone_r, zone) in &zones {
            let zone_pos = zone_tf.translation.truncate();
            if top_pos.distance(zone_pos) < top_radius + zone_r.0 {
                in_zone = true;
                best_mult = best_mult.max(zone.multiplier);
                best_dur = best_dur.max(zone.duration);
            }
        }

        if in_zone {
            if effect.multiplier <= 1.0 {
                info!("SpeedBoost ACTIVATED: multiplier={:.2}, duration={:.1}s", best_mult, best_dur);
            }
            effect.expires_at = now + best_dur as f64;
            effect.multiplier = best_mult;
        }
    }
}

/// Reset expired speed boost effects to neutral (multiplier 1.0).
fn speed_boost_tick(
    time: Res<Time>,
    mut query: Query<&mut SpeedBoostEffect>,
) {
    let now = time.elapsed_secs_f64();
    for mut effect in &mut query {
        if now >= effect.expires_at {
            effect.multiplier = 1.0;
        }
    }
}

/// Damage boost: tops overlapping a DamageBoostZone get a damage multiplier.
/// Mutates the always-present DamageBoostActive directly (no deferred Commands).
fn damage_boost_system(
    zones: Query<(&Transform, &CollisionRadius, &DamageBoostZone)>,
    mut tops: Query<(&Transform, &TopEffectiveStats, &mut DamageBoostActive), With<Top>>,
) {
    for (top_tf, top_stats, mut boost) in &mut tops {
        let top_pos = top_tf.translation.truncate();
        let top_radius = top_stats.0.radius.0;
        let mut in_zone = false;
        let mut best_mult = 1.0_f32;

        for (zone_tf, zone_r, zone) in &zones {
            let zone_pos = zone_tf.translation.truncate();
            if top_pos.distance(zone_pos) < top_radius + zone_r.0 {
                in_zone = true;
                best_mult = best_mult.max(zone.multiplier);
            }
        }

        if in_zone {
            if boost.multiplier <= 1.0 {
                info!("DamageBoost ACTIVATED: multiplier={:.2}", best_mult);
            }
            boost.multiplier = best_mult;
        } else {
            boost.multiplier = 1.0;
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
