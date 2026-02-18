use bevy::prelude::*;

use super::components::*;
use super::events::{CollisionMessage, GameEvent};
use super::stats::types::DamageKind;
use crate::config::tuning::Tuning;

/// EventGenerateSet: convert collisions into DealDamage events (base damage only).
/// DamageBoostActive is applied centrally in apply_damage_events.
pub fn generate_collision_damage(
    tuning: Res<Tuning>,
    mut collision_events: MessageReader<CollisionMessage>,
    mut out_events: MessageWriter<GameEvent>,
) {
    for event in collision_events.read() {
        let damage = tuning.collision_damage_k * event.impulse;

        out_events.write(GameEvent::DealDamage {
            src: Some(event.a),
            dst: event.b,
            amount: damage,
            kind: DamageKind::Collision,
            tags: vec!["collision".into()],
        });
        out_events.write(GameEvent::DealDamage {
            src: Some(event.b),
            dst: event.a,
            amount: damage,
            kind: DamageKind::Collision,
            tags: vec!["collision".into()],
        });
    }
}

/// EventApplySet: apply DealDamage events to SpinHp.
pub fn apply_damage_events(
    mut events: MessageReader<GameEvent>,
    mut tops: Query<(&mut SpinHpCurrent, &TopEffectiveStats, &DamageBoostActive), With<Top>>,
) {
    for event in events.read() {
        if let GameEvent::DealDamage {
            src,
            dst,
            amount,
            kind: _,
            tags: _,
        } = event
        {
            let mut amount = *amount;

            // Apply source damage output multiplier + damage boost zone
            if let Some(src_entity) = src {
                if let Ok((_, src_stats, dmg_boost)) = tops.get(*src_entity) {
                    let before = amount;
                    amount *= src_stats.0.damage_out_mult.0;
                    amount *= dmg_boost.multiplier;
                    if dmg_boost.multiplier > 1.001 {
                        info!(
                            "[DamageBoost] base={:.2} * out_mult={:.2} * boost={:.2} = {:.2}",
                            before, src_stats.0.damage_out_mult.0, dmg_boost.multiplier, amount
                        );
                    }
                }
            }

            // Apply destination damage intake multiplier
            if let Ok((mut spin, dst_stats, _)) = tops.get_mut(*dst) {
                amount *= dst_stats.0.damage_in_mult.0;
                amount = amount.max(0.0);
                spin.0 = spin.0.sub_clamped(amount);
            }
        }
    }
}

/// EventApplySet: apply control effects.
pub fn apply_control_events(
    mut events: MessageReader<GameEvent>,
    mut tops: Query<(&mut ControlState, &TopEffectiveStats), With<Top>>,
) {
    for event in events.read() {
        if let GameEvent::ApplyControl { src: _, dst, control } = event {
            if let Ok((mut ctrl_state, stats)) = tops.get_mut(*dst) {
                ctrl_state.apply_control(*control, stats.0.control_multiplier);
            }
        }
    }
}

/// Resolve Top–Top collision physics (velocity exchange).
pub fn resolve_top_collisions(
    tuning: Res<Tuning>,
    mut events: MessageReader<CollisionMessage>,
    mut tops: Query<(&mut Transform, &mut Velocity, &TopEffectiveStats), With<Top>>,
) {
    let e = tuning.top_collisions_restitution.clamp(0.0, 1.0);

    for event in events.read() {
        if let Ok([mut top_a, mut top_b]) = tops.get_many_mut([event.a, event.b]) {
            // Treat stability as "heaviness": higher stability => lower inv_mass
            let inv_mass_a = 1.0 / (1.0 + top_a.2 .0.stability.max(0.0));
            let inv_mass_b = 1.0 / (1.0 + top_b.2 .0.stability.max(0.0));
            let inv_mass_sum = inv_mass_a + inv_mass_b;

            if inv_mass_sum <= 0.0 {
                continue;
            }

            let n = event.normal;

            // Relative velocity along normal
            let v_rel = top_a.1 .0 - top_b.1 .0;
            let v_rel_n = v_rel.dot(n);

            // Only resolve if they are moving toward each other along the normal
            if v_rel_n <= 0.0 {
                continue;
            }

            // Impulse magnitude (standard)
            let j = (1.0 + e) * v_rel_n / inv_mass_sum;

            top_a.1 .0 -= j * inv_mass_a * n;
            top_b.1 .0 += j * inv_mass_b * n;

            // Separate overlap (keep your current logic, but compute normal from positions for stability)
            let pos_a = top_a.0.translation.truncate();
            let pos_b = top_b.0.translation.truncate();
            let delta = pos_b - pos_a;
            let dist = delta.length();
            let min_dist = top_a.2 .0.radius.0 + top_b.2 .0.radius.0;

            if dist < min_dist && dist > 0.0 {
                let overlap = min_dist - dist;
                let sep_n = delta / dist;
                // split by mass (heavier moves less)
                let move_a = overlap * (inv_mass_a / inv_mass_sum);
                let move_b = overlap * (inv_mass_b / inv_mass_sum);

                top_a.0.translation.x -= sep_n.x * move_a;
                top_a.0.translation.y -= sep_n.y * move_a;
                top_b.0.translation.x += sep_n.x * move_b;
                top_b.0.translation.y += sep_n.y * move_b;
            }

        }
    }
}

/// Fire ranged weapon projectiles (auto-fires when cooldown expires).
pub fn fire_ranged_weapons(
    tuning: Res<Tuning>,
    mut query: Query<
        (
            Entity,
            &Transform,
            &RotationAngle,
            &TopBuild,
            &TopEffectiveStats,
            &mut RangedFireTimer,
        ),
        With<Top>,
    >,
    mut events: MessageWriter<GameEvent>,
) {
    for (entity, transform, angle, build, stats, mut timer) in &mut query {
        timer.0 -= tuning.dt;

        if timer.0 > 0.0 {
            continue;
        }

        if let Some(ranged) = &build.0.weapon.ranged {
            let fire_rate = ranged.fire_rate * stats.0.fire_rate_mult.0;
            timer.0 = 1.0 / fire_rate.max(0.1);

            let pos = transform.translation.truncate();
            let dir = Vec2::new(angle.0 .0.cos(), angle.0 .0.sin());
            let wid = build.0.weapon.id.clone();

            if ranged.burst_count <= 1 && ranged.spread_angle <= 0.0 {
                events.write(GameEvent::SpawnProjectile {
                    src: entity,
                    position: pos + dir * stats.0.radius.0,
                    direction: dir,
                    speed: ranged.projectile_speed,
                    damage: ranged.projectile_damage,
                    radius: ranged.projectile_radius,
                    lifetime: ranged.lifetime.0,
                    weapon_id: wid,
                });
            } else {
                let count = ranged.burst_count.max(1);
                let total_spread = ranged.spread_angle;
                let step = if count > 1 {
                    total_spread / (count - 1) as f32
                } else {
                    0.0
                };
                let start_angle = angle.0 .0 - total_spread / 2.0;

                for i in 0..count {
                    let a = start_angle + step * i as f32;
                    let d = Vec2::new(a.cos(), a.sin());
                    events.write(GameEvent::SpawnProjectile {
                        src: entity,
                        position: pos + d * stats.0.radius.0,
                        direction: d,
                        speed: ranged.projectile_speed,
                        damage: ranged.projectile_damage,
                        radius: ranged.projectile_radius,
                        lifetime: ranged.lifetime.0,
                        weapon_id: wid.clone(),
                    });
                }
            }
        }
    }
}

/// Component to track ranged weapon fire cooldown.
#[derive(Component)]
pub struct RangedFireTimer(pub f32);

impl Default for RangedFireTimer {
    fn default() -> Self {
        Self(0.0)
    }
}

/// Detect melee hits.
pub fn detect_melee_hits(
    tuning: Res<Tuning>,
    mut attackers: Query<
        (
            Entity,
            &Transform,
            &RotationAngle,
            &TopBuild,
            &TopEffectiveStats,
            &Velocity,
            &mut MeleeHitTracker,
        ),
        With<Top>,
    >,
    targets: Query<(Entity, &Transform, &TopEffectiveStats), With<Top>>,
    mut events: MessageWriter<GameEvent>,
) {
    for (atk_entity, atk_tf, atk_angle, atk_build, atk_stats, atk_vel, mut tracker) in
        &mut attackers
    {
        let melee = match &atk_build.0.weapon.melee {
            Some(m) => m,
            None => continue,
        };

        let atk_pos = atk_tf.translation.truncate();
        let weapon_dir = Vec2::new(atk_angle.0 .0.cos(), atk_angle.0 .0.sin());

        for (tgt_entity, tgt_tf, tgt_stats) in &targets {
            if atk_entity == tgt_entity {
                continue;
            }

            if !tracker.can_hit(tgt_entity) {
                continue;
            }

            let tgt_pos = tgt_tf.translation.truncate();
            let to_target = tgt_pos - atk_pos;
            let dist = to_target.length();

            let reach = atk_stats.0.radius.0 + melee.hitbox_radius;
            if dist > reach + tgt_stats.0.radius.0 {
                continue;
            }

            if dist > 0.0 {
                let target_dir = to_target / dist;
                let angle = weapon_dir.dot(target_dir).acos();
                if angle > melee.hitbox_angle / 2.0 {
                    continue;
                }
            }

            tracker.register_hit(tgt_entity, melee.hit_cooldown);

            let mut damage = melee.base_damage;
            if tuning.melee_speed_scale_k > 0.0 {
                let rel_speed = atk_vel.0.length();
                damage *= 1.0 + tuning.melee_speed_scale_k * rel_speed;
            }

            events.write(GameEvent::DealDamage {
                src: Some(atk_entity),
                dst: tgt_entity,
                amount: damage,
                kind: DamageKind::Melee,
                tags: vec![],
            });

            if let Some(control) = melee.hit_control {
                events.write(GameEvent::ApplyControl {
                    src: Some(atk_entity),
                    dst: tgt_entity,
                    control,
                });
            }
        }
    }
}
