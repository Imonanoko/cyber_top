use bevy::prelude::*;

use crate::game::components::*;
use crate::game::events::GameEvent;

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
