use bevy::prelude::*;

use super::components::*;
use crate::config::tuning::Tuning;

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
