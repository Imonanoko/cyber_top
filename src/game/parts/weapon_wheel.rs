use serde::{Deserialize, Serialize};

use crate::game::stats::types::{AimMode, ControlEffect, Seconds, WeaponKind};

/// Melee weapon specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeleeSpec {
    pub base_damage: f32,
    /// Cooldown between hits on the same target (seconds).
    pub hit_cooldown: f32,
    /// Max hits per full rotation (optional, 0 = unlimited).
    pub max_hits_per_rotation: u32,
    /// Hitbox radius from top center.
    pub hitbox_radius: f32,
    /// Hitbox angular span (radians).
    pub hitbox_angle: f32,
    /// Control effect on hit (optional).
    pub hit_control: Option<ControlEffect>,
    /// Visual spin rate multiplier (1.0 = default, higher = faster rotation).
    pub spin_rate_multiplier: f32,
    /// Blade visual length (world units).
    pub blade_len: f32,
    /// Blade visual thickness (world units).
    pub blade_thick: f32,
}

impl Default for MeleeSpec {
    fn default() -> Self {
        Self {
            base_damage: 5.5,
            hit_cooldown: 0.5,
            max_hits_per_rotation: 0,
            hitbox_radius: 2.5,
            hitbox_angle: std::f32::consts::FRAC_PI_3, // 60 degrees
            hit_control: None,
            spin_rate_multiplier: 1.0,
            blade_len: 2.3,
            blade_thick: 0.3,
        }
    }
}

/// Ranged weapon specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangedSpec {
    pub projectile_damage: f32,
    /// Shots per second.
    pub fire_rate: f32,
    /// Number of projectiles per burst.
    pub burst_count: u32,
    /// Spread angle (radians, 0 = single line).
    pub spread_angle: f32,
    /// Knockback distance on hit.
    pub knockback_distance: f32,
    /// Projectile radius.
    pub projectile_radius: f32,
    /// Control effect on hit duration.
    pub control_duration: Seconds,
    /// Projectile lifetime / range (seconds).
    pub lifetime: Seconds,
    /// Projectile speed.
    pub projectile_speed: f32,
    pub aim_mode: AimMode,
    /// Visual spin rate multiplier (1.0 = default, higher = faster rotation).
    pub spin_rate_multiplier: f32,
    pub barrel_len: f32,
    pub barrel_thick: f32
}

impl Default for RangedSpec {
    fn default() -> Self {
        Self {
            projectile_damage: 7.0,
            fire_rate: 3.0,
            burst_count: 1,
            spread_angle: 0.0,
            knockback_distance: 0.0,
            projectile_radius: 0.3,
            control_duration: Seconds(0.0),
            lifetime: Seconds(2.0),
            projectile_speed: 15.0,
            aim_mode: AimMode::FollowSpin,
            spin_rate_multiplier: 0.3,
            barrel_len: 1.0,
            barrel_thick: 0.3
        }
    }
}

/// Weapon wheel specification (the weapon part).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponWheelSpec {
    pub id: String,
    pub name: String,
    pub kind: WeaponKind,
    pub melee: Option<MeleeSpec>,
    pub ranged: Option<RangedSpec>,
}

impl WeaponWheelSpec {
    /// Get the effective spin rate multiplier from the active spec.
    /// Hybrid: uses the max of both. No spec: defaults to 1.0.
    pub fn spin_rate_multiplier(&self) -> f32 {
        match (&self.melee, &self.ranged) {
            (Some(m), Some(r)) => m.spin_rate_multiplier.max(r.spin_rate_multiplier),
            (Some(m), None) => m.spin_rate_multiplier,
            (None, Some(r)) => r.spin_rate_multiplier,
            (None, None) => 1.0,
        }
    }
}

impl Default for WeaponWheelSpec {
    fn default() -> Self {
        Self {
            id: "default_melee".into(),
            name: "Basic Blade".into(),
            kind: WeaponKind::Melee,
            melee: Some(MeleeSpec::default()),
            ranged: None,
        }
    }
}
