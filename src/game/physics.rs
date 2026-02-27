use bevy::prelude::*;

use super::components::*;
use crate::config::tuning::Tuning;
use crate::game::stats::types::AimMode;

/// PhysicsSet: integrate velocity → position, update rotation angle.
pub fn integrate_physics(
    tuning: Res<Tuning>,
    mut query: Query<(&mut Transform, &Velocity, &mut RotationAngle, &TopBuild, &SpeedBoostEffect), With<Top>>,
    mut tick: Local<u32>,
) {
    *tick = tick.wrapping_add(1);
    let log_this_tick = *tick % 60 == 0;

    let dt = tuning.dt;
    for (mut transform, vel, mut angle, build, speed_boost) in &mut query {
        let eff_vel = vel.0 * speed_boost.multiplier;

        if log_this_tick && speed_boost.multiplier > 1.001 {
            info!(
                "[SpeedBoost] vel_speed={:.2}  eff_speed={:.2}  multiplier={:.2}",
                vel.0.length(),
                eff_vel.length(),
                speed_boost.multiplier
            );
        }

        transform.translation.x += eff_vel.x * dt;
        transform.translation.y += eff_vel.y * dt;

        if eff_vel.length_squared() > 0.001 {
            let weapon_mult = build.0.weapon.spin_rate_multiplier();
            let spin_rate = eff_vel.length() * tuning.spin_visual_k * weapon_mult;
            angle.0 = angle.0.advance(spin_rate * dt);
        }

        // Sync visual rotation so weapon child entities rotate with the top
        transform.rotation = Quat::from_rotation_z(angle.0 .0);
    }
}

/// Integrate projectile movement and tick lifetime.
pub fn integrate_projectiles(
    tuning: Res<Tuning>,
    mut query: Query<(&mut Transform, &Velocity, &mut Lifetime), With<ProjectileMarker>>,
) {
    let dt = tuning.dt;
    for (mut transform, vel, mut lifetime) in &mut query {
        transform.translation.x += vel.0.x * dt;
        transform.translation.y += vel.0.y * dt;
        lifetime.0 = lifetime.0.dec(dt);
    }
}

/// Apply natural spin drain (idle).
pub fn spin_drain(
    tuning: Res<Tuning>,
    mut query: Query<(&mut SpinHpCurrent, &TopEffectiveStats), With<Top>>,
) {
    let dt = tuning.dt;
    for (mut spin, stats) in &mut query {
        let drain = stats.0.spin_drain_idle_per_sec * dt;
        spin.0 = spin.0.sub_clamped(drain);
    }
}

/// Tick control state timers.
pub fn tick_control_state(tuning: Res<Tuning>, mut query: Query<&mut ControlState, With<Top>>) {
    let dt = tuning.dt;
    for mut control in &mut query {
        control.tick(dt);
    }
}

/// Tick melee hit trackers.
pub fn tick_melee_trackers(tuning: Res<Tuning>, mut query: Query<&mut MeleeHitTracker>) {
    let dt = tuning.dt;
    for mut tracker in &mut query {
        tracker.tick(dt);
    }
}

/// PhysicsSet: for SeekNearestTarget ranged weapons, rotate the weapon visual to face
/// the nearest enemy and store the aim angle in `WeaponAimAngle`.
/// Must run AFTER `integrate_physics` so `RotationAngle` (spin) is up-to-date.
pub fn update_seek_weapon_visual(
    tops: Query<(Entity, &Transform, &TopBuild, &RotationAngle, &TopEffectiveStats, &Children), With<Top>>,
    mut aim_angles: Query<&mut WeaponAimAngle>,
    mut weapon_visuals: Query<&mut Transform, (With<WeaponVisual>, Without<Top>)>,
) {
    // Snapshot all top positions (avoids borrow conflict with mutable queries below).
    let positions: Vec<(Entity, Vec2)> = tops.iter()
        .map(|(e, tf, _, _, _, _)| (e, tf.translation.truncate()))
        .collect();

    for (self_entity, self_tf, build, spin_angle, self_stats, children) in &tops {
        let ranged = match &build.0.weapon.ranged {
            Some(r) if r.aim_mode == AimMode::SeekNearestTarget => r,
            _ => continue,
        };

        let self_pos = self_tf.translation.truncate();
        let nearest = positions.iter()
            .filter(|(e, _)| *e != self_entity)
            .min_by(|(_, a), (_, b)| {
                a.distance(self_pos)
                    .partial_cmp(&b.distance(self_pos))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some((_, target_pos)) = nearest else { continue };
        let dir = *target_pos - self_pos;
        if dir.length_squared() < 0.001 {
            continue;
        }

        let world_angle = dir.y.atan2(dir.x);

        // Store world-space aim angle so fire_ranged_weapons can read it.
        if let Ok(mut aim) = aim_angles.get_mut(self_entity) {
            aim.0 = world_angle;
        }

        // Update WeaponVisual child local transform to face the target.
        // Local angle counteracts parent spin so the weapon points in world_angle.
        let local_angle = world_angle - spin_angle.0 .0;
        let barrel_offset = self_stats.0.radius.0 + ranged.barrel_len * 0.5;

        for child in children.iter() {
            if let Ok(mut vis_tf) = weapon_visuals.get_mut(child) {
                vis_tf.translation.x = barrel_offset * local_angle.cos();
                vis_tf.translation.y = barrel_offset * local_angle.sin();
                vis_tf.translation.z = 0.5;
                vis_tf.rotation = Quat::from_rotation_z(local_angle);
            }
        }
    }
}
