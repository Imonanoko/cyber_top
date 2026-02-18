use bevy::prelude::*;

use crate::config::tuning::Tuning;
use crate::game::components::*;
use crate::game::events::GameEvent;

/// Despawn projectiles that leave the arena boundary.
pub fn despawn_projectiles_outside_arena(
    mut commands: Commands,
    tuning: Res<Tuning>,
    arena_r_res: Option<Res<ArenaRadius>>,
    query: Query<(Entity, &Transform, &CollisionRadius), With<ProjectileMarker>>,
) {
    let arena_r = arena_r_res.map(|r| r.0).unwrap_or(tuning.arena_radius);
    for (entity, transform, radius) in &query {
        let pos = transform.translation.truncate();
        if pos.length() > arena_r + radius.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Wall reflection system — handles Top bouncing off the circular arena boundary.
/// This is the authoritative wall reflection that also generates wall damage events.
pub fn wall_reflection(
    tuning: Res<Tuning>,
    arena_r_res: Option<Res<ArenaRadius>>,
    mut query: Query<(Entity, &mut Transform, &mut Velocity, &TopEffectiveStats), With<Top>>,
    mut events: MessageWriter<GameEvent>,
) {
    let arena_r = arena_r_res.map(|r| r.0).unwrap_or(tuning.arena_radius);
    let damping = tuning.wall_bounce_damping.clamp(0.0, 1.0);

    for (entity, mut transform, mut vel, stats) in &mut query {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        let top_radius = stats.0.radius.0;
        let dist = pos.length();
        let boundary = arena_r - top_radius;

        if dist > boundary && dist > 0.0 {
            let normal = pos / dist;
            let overshoot = dist - boundary;

            // Push back inside
            transform.translation.x -= normal.x * overshoot;
            transform.translation.y -= normal.y * overshoot;

            // Reflect velocity
            let dot = vel.0.dot(normal);
            if dot > 0.0 {
                vel.0 -= 2.0 * dot * normal;
                vel.0 *= damping;

                // Generate wall damage event (fixed amount, not speed-scaled)
                if tuning.wall_damage_k > 0.0 {
                    let wall_dmg = tuning.wall_damage_k;
                    events.write(GameEvent::DealDamage {
                        src: None,
                        dst: entity,
                        amount: wall_dmg,
                        kind: crate::game::stats::types::DamageKind::Wall,
                        tags: vec!["wall_hit".into()],
                    });
                }

                // Spin drain on wall hit
                // (handled separately in spin_drain system reading events)
            }
        }
    }
}
