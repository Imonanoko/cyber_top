use bevy::prelude::*;

use super::components::*;
use super::events::{CollisionMessage, GameEvent};
use super::stats::types::DamageKind;
use crate::config::tuning::Tuning;

/// Collision detection: Top–Top, Top–Wall, Top–Obstacle, Projectile–Top.
pub fn detect_collisions(
    tuning: Res<Tuning>,
    tops: Query<(Entity, &Transform, &Velocity, &TopEffectiveStats), With<Top>>,
    obstacles: Query<
        (Entity, &Transform, &CollisionRadius, &ObstacleBehavior),
        With<ObstacleMarker>,
    >,
    projectiles: Query<
        (Entity, &Transform, &CollisionRadius, &ProjectileOwner, &ProjectileDamage),
        With<ProjectileMarker>,
    >,
    mut collision_events: MessageWriter<CollisionMessage>,
    mut events: MessageWriter<GameEvent>,
) {
    let top_list: Vec<_> = tops.iter().collect();

    // Top–Top collisions
    for i in 0..top_list.len() {
        for j in (i + 1)..top_list.len() {
            let (e_a, tf_a, vel_a, stats_a) = &top_list[i];
            let (e_b, tf_b, vel_b, stats_b) = &top_list[j];

            let pos_a = tf_a.translation.truncate();
            let pos_b = tf_b.translation.truncate();
            let dist = pos_a.distance(pos_b);
            let min_dist = stats_a.0.radius.0 + stats_b.0.radius.0;

            if dist < min_dist && dist > 0.0 {
                let normal = (pos_b - pos_a) / dist;
                let rel_vel = vel_a.0 - vel_b.0;
                let impulse = rel_vel.dot(normal);

                if impulse > 0.0 {
                    collision_events.write(CollisionMessage {
                        a: *e_a,
                        b: *e_b,
                        impulse,
                        normal,
                    });
                }
            }
        }

        // Top–Wall collision is handled by circle::wall_reflection (PhysicsSet).
        // No wall damage here to avoid double counting.

        // Top–Obstacle collisions
        let (entity, tf, _vel, stats) = &top_list[i];
        for (obs_entity, obs_tf, obs_radius, obs_behavior) in &obstacles {
            let pos_top = tf.translation.truncate();
            let pos_obs = obs_tf.translation.truncate();
            let dist = pos_top.distance(pos_obs);
            let min_dist = stats.0.radius.0 + obs_radius.0;

            if dist < min_dist {
                match obs_behavior.0 {
                    super::stats::types::CollisionBehavior::DamageOnHit => {
                        events.write(GameEvent::DealDamage {
                            src: Some(obs_entity),
                            dst: *entity,
                            amount: tuning.obstacle_damage,
                            kind: DamageKind::Obstacle,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    // Projectile–Top collisions
    for (proj_entity, proj_tf, proj_radius, proj_owner, proj_dmg) in &projectiles {
        let proj_pos = proj_tf.translation.truncate();

        for (top_entity, top_tf, _, top_stats) in &top_list {
            // Don't hit owner
            if *top_entity == proj_owner.0 {
                continue;
            }

            let top_pos = top_tf.translation.truncate();
            let dist = proj_pos.distance(top_pos);
            let min_dist = proj_radius.0 + top_stats.0.radius.0;

            if dist < min_dist {
                events.write(GameEvent::DealDamage {
                    src: Some(proj_owner.0),
                    dst: *top_entity,
                    amount: proj_dmg.0,
                    kind: DamageKind::Projectile,
                });
                events.write(GameEvent::DespawnEntity {
                    entity: proj_entity,
                });
            }
        }
    }
}
