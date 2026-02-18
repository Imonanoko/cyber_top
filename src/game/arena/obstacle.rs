use bevy::prelude::*;

use crate::game::components::*;
use crate::game::events::GameEvent;

/// Bounce tops off static obstacles (elastic reflection + push-out physics only).
/// Damage is handled by detect_collisions via ObstacleMarker/DamageOnHit.
/// Runs in PhysicsSet so it can mutate Transform/Velocity.
pub fn static_obstacle_bounce(
    mut tops: Query<(&mut Transform, &mut Velocity, &TopEffectiveStats), With<Top>>,
    obstacles: Query<(&Transform, &CollisionRadius), (With<StaticObstacle>, Without<Top>)>,
) {
    for (mut top_tf, mut vel, stats) in &mut tops {
        let top_pos = top_tf.translation.truncate();
        let top_radius = stats.0.radius.0;

        for (obs_tf, obs_radius) in &obstacles {
            let obs_pos = obs_tf.translation.truncate();
            let dist = top_pos.distance(obs_pos);
            let min_dist = top_radius + obs_radius.0;

            if dist < min_dist && dist > 0.0 {
                let normal = (top_pos - obs_pos) / dist;
                let overshoot = min_dist - dist;

                // Push top out of overlap
                top_tf.translation.x += normal.x * overshoot;
                top_tf.translation.y += normal.y * overshoot;

                // Elastic reflection: only if moving toward obstacle
                let dot = vel.0.dot(-normal);
                if dot > 0.0 {
                    vel.0 = vel.0 - 2.0 * vel.0.dot(-normal) * (-normal);
                }
            }
        }
    }
}

/// Spawn obstacle entities from SpawnObstacle events.
pub fn spawn_obstacles(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    time: Res<Time>,
) {
    for event in events.read() {
        if let GameEvent::SpawnObstacle {
            src,
            position,
            radius,
            ttl,
            behavior,
        } = event
        {
            let expires_at = time.elapsed_secs_f64() + *ttl as f64;

            commands.spawn((
                ObstacleMarker,
                Transform::from_translation(Vec3::new(position.x, position.y, 0.0)),
                CollisionRadius(*radius),
                ObstacleOwner(*src),
                ObstacleBehavior(*behavior),
                ExpiresAt(expires_at),
            ));
        }
    }
}

/// Spawn projectile entities from SpawnProjectile events (with visible mesh or sprite).
pub fn spawn_projectiles(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    proj_assets: Res<ProjectileAssets>,
) {
    for event in events.read() {
        if let GameEvent::SpawnProjectile {
            src,
            position,
            direction,
            speed,
            damage,
            radius,
            lifetime,
            weapon_id,
        } = event
        {
            let tf = Transform::from_translation(Vec3::new(position.x, position.y, 0.5));
            let mut entity = commands.spawn((
                ProjectileMarker,
                Velocity(*direction * *speed),
                CollisionRadius(*radius),
                ProjectileOwner(*src),
                ProjectileDamage(*damage),
                Lifetime(crate::game::stats::types::Seconds(*lifetime)),
            ));

            if let Some(sprite_handle) = proj_assets.sprites.get(weapon_id) {
                let diameter = *radius * 2.0;
                entity.insert((
                    Sprite {
                        image: sprite_handle.clone(),
                        custom_size: Some(Vec2::new(diameter, diameter)),
                        ..default()
                    },
                    tf,
                ));
            } else {
                entity.insert((
                    Mesh2d(proj_assets.mesh.clone()),
                    MeshMaterial2d(proj_assets.material.clone()),
                    tf.with_scale(Vec3::splat(*radius)),
                ));
            }
        }
    }
}

/// CleanupSet: despawn obstacles and projectiles that have expired.
pub fn cleanup_ttl(
    mut commands: Commands,
    time: Res<Time>,
    obstacles: Query<(Entity, &ExpiresAt), With<ObstacleMarker>>,
    projectiles: Query<(Entity, &Lifetime), With<ProjectileMarker>>,
) {
    let now = time.elapsed_secs_f64();

    for (entity, expires) in &obstacles {
        if now >= expires.0 {
            commands.entity(entity).despawn();
        }
    }

    for (entity, lifetime) in &projectiles {
        if lifetime.0.is_expired() {
            commands.entity(entity).despawn();
        }
    }
}

/// Handle DespawnEntity events.
pub fn handle_despawn_events(mut commands: Commands, mut events: MessageReader<GameEvent>) {
    for event in events.read() {
        if let GameEvent::DespawnEntity { entity } = event {
            commands.entity(*entity).try_despawn();
        }
    }
}
