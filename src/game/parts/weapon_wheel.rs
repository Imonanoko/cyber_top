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
}

impl Default for MeleeSpec {
    fn default() -> Self {
        Self {
            base_damage: 5.0,
            hit_cooldown: 0.5,
            max_hits_per_rotation: 0,
            hitbox_radius: 1.0,
            hitbox_angle: std::f32::consts::FRAC_PI_3, // 60 degrees
            hit_control: None,
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
}

impl Default for RangedSpec {
    fn default() -> Self {
        Self {
            projectile_damage: 3.0,
            fire_rate: 2.0,
            burst_count: 1,
            spread_angle: 0.0,
            knockback_distance: 0.0,
            projectile_radius: 0.15,
            control_duration: Seconds(0.0),
            lifetime: Seconds(2.0),
            projectile_speed: 10.0,
            aim_mode: AimMode::FollowSpin,
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
