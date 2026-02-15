use bevy::prelude::*;

use crate::config::tuning::Tuning;
use crate::game::components::*;
use crate::game::events::GameEvent;

/// Wall reflection system — handles Top bouncing off the circular arena boundary.
/// This is the authoritative wall reflection that also generates wall damage events.
pub fn wall_reflection(
    tuning: Res<Tuning>,
    mut query: Query<(Entity, &mut Transform, &mut Velocity, &TopEffectiveStats), With<Top>>,
    mut events: MessageWriter<GameEvent>,
) {
    let arena_r = tuning.arena_radius;
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

                // Clamp speed after reflection
                let speed = vel.0.length();
                if speed > tuning.max_speed {
                    vel.0 = vel.0.normalize_or_zero() * tuning.max_speed;
                }

                // Generate wall damage event
                if tuning.wall_damage_k > 0.0 {
                    let wall_dmg = tuning.wall_damage_k * dot;
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
