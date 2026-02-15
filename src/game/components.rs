use bevy::prelude::*;

use super::parts::Build;
use super::stats::effective::EffectiveStats;
use super::stats::types::{AngleRad, CollisionBehavior, ControlEffect, Seconds, SpinHp};

// ── Marker components ───────────────────────────────────────────────

#[derive(Component)]
pub struct Top;

#[derive(Component)]
pub struct ProjectileMarker;

#[derive(Component)]
pub struct ObstacleMarker;

#[derive(Component)]
pub struct PlayerControlled;

#[derive(Component)]
pub struct AiControlled;

// ── Game phase state ────────────────────────────────────────────────

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GamePhase {
    #[default]
    Aiming,
    Battle,
    GameOver,
}

// ── Launch aiming ───────────────────────────────────────────────────

#[derive(Component)]
pub struct LaunchAim {
    pub angle: f32,
    pub confirmed: bool,
}

impl Default for LaunchAim {
    fn default() -> Self {
        Self {
            angle: 0.0,
            confirmed: false,
        }
    }
}

/// Marker for the aiming arrow entity so we can despawn it later.
#[derive(Component)]
pub struct AimArrow;

// ── Top runtime state ───────────────────────────────────────────────

#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct RotationAngle(pub AngleRad);

#[derive(Component)]
pub struct SpinHpCurrent(pub SpinHp);

#[derive(Component)]
pub struct TopEffectiveStats(pub EffectiveStats);

#[derive(Component)]
pub struct TopBuild(pub Build);

/// Active control effects on a Top.
#[derive(Component, Default)]
pub struct ControlState {
    pub stun_remaining: Seconds,
    pub slow_remaining: Seconds,
    pub slow_ratio: f32,
}

impl ControlState {
    pub fn is_stunned(&self) -> bool {
        !self.stun_remaining.is_expired()
    }

    pub fn is_slowed(&self) -> bool {
        !self.slow_remaining.is_expired()
    }

    pub fn tick(&mut self, dt: f32) {
        self.stun_remaining = self.stun_remaining.dec(dt);
        self.slow_remaining = self.slow_remaining.dec(dt);
    }

    pub fn apply_control(&mut self, control: ControlEffect, control_multiplier: f32) {
        let reduced = control.apply_reduction(control_multiplier);
        match reduced {
            ControlEffect::Stun { duration } => {
                if duration.0 > self.stun_remaining.0 {
                    self.stun_remaining = duration;
                }
            }
            ControlEffect::Slow { duration, ratio } => {
                if duration.0 > self.slow_remaining.0 {
                    self.slow_remaining = duration;
                    self.slow_ratio = ratio;
                }
            }
            ControlEffect::Knockback { .. } => {
                // Knockback is applied as velocity impulse, handled in physics
            }
        }
    }
}

/// Status effect instances active on a Top.
#[derive(Component, Default)]
pub struct StatusEffects {
    pub effects: Vec<StatusEffectInstance>,
}

#[derive(Debug, Clone)]
pub struct StatusEffectInstance {
    pub kind: super::events::StatusEffectKind,
    pub remaining: Seconds,
    pub magnitude: f32,
}

impl StatusEffects {
    pub fn tick(&mut self, dt: f32) {
        for effect in &mut self.effects {
            effect.remaining = effect.remaining.dec(dt);
        }
        self.effects.retain(|e| !e.remaining.is_expired());
    }
}

// ── Projectile state ────────────────────────────────────────────────

#[derive(Component)]
pub struct ProjectileDamage(pub f32);

#[derive(Component)]
pub struct ProjectileOwner(pub Entity);

#[derive(Component)]
pub struct Lifetime(pub Seconds);

#[derive(Component)]
pub struct CollisionRadius(pub f32);

// ── Obstacle state ──────────────────────────────────────────────────

#[derive(Component)]
pub struct ObstacleOwner(pub Option<Entity>);

#[derive(Component)]
pub struct ObstacleBehavior(pub CollisionBehavior);

#[derive(Component)]
pub struct ExpiresAt(pub f64);

// ── Melee tracking ──────────────────────────────────────────────────

/// Tracks per-target hit cooldowns for melee weapons.
#[derive(Component, Default)]
pub struct MeleeHitTracker {
    /// (target entity, time until can hit again)
    pub cooldowns: Vec<(Entity, f32)>,
}

impl MeleeHitTracker {
    pub fn can_hit(&self, target: Entity) -> bool {
        !self.cooldowns.iter().any(|(e, t)| *e == target && *t > 0.0)
    }

    pub fn register_hit(&mut self, target: Entity, cooldown: f32) {
        self.cooldowns.push((target, cooldown));
    }

    pub fn tick(&mut self, dt: f32) {
        for (_, t) in &mut self.cooldowns {
            *t -= dt;
        }
        self.cooldowns.retain(|(_, t)| *t > 0.0);
    }
}
