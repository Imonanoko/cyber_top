use bevy::prelude::*;

use super::components::*;
use crate::config::tuning::Tuning;

/// PhysicsSet: integrate velocity → position, update rotation angle.
pub fn integrate_physics(
    tuning: Res<Tuning>,
    mut query: Query<(&mut Transform, &Velocity, &mut RotationAngle), With<Top>>,
) {
    let dt = tuning.dt;
    for (mut transform, vel, mut angle) in &mut query {
        transform.translation.x += vel.0.x * dt;
        transform.translation.y += vel.0.y * dt;

        if vel.0.length_squared() > 0.001 {
            let spin_rate = vel.0.length() * 2.0;
            angle.0 = angle.0.advance(spin_rate * dt);
        }
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

/// Tick status effects.
pub fn tick_status_effects(tuning: Res<Tuning>, mut query: Query<&mut StatusEffects, With<Top>>) {
    let dt = tuning.dt;
    for mut effects in &mut query {
        effects.tick(dt);
    }
}

/// Tick melee hit trackers.
pub fn tick_melee_trackers(tuning: Res<Tuning>, mut query: Query<&mut MeleeHitTracker>) {
    let dt = tuning.dt;
    for mut tracker in &mut query {
        tracker.tick(dt);
    }
}
