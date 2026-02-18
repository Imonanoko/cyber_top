use std::f32::consts::TAU;
use std::ops::Mul;

use serde::{Deserialize, Serialize};

// ── Newtypes ────────────────────────────────────────────────────────

/// Spin RPM = HP. Always clamped to [0, max].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct SpinHp(pub f32);

impl SpinHp {
    pub fn new(v: f32) -> Self {
        debug_assert!(v.is_finite(), "SpinHp must be finite");
        Self(v.max(0.0))
    }

    pub fn add_clamped(self, delta: f32, max: f32) -> Self {
        let v = (self.0 + delta).clamp(0.0, max);
        debug_assert!(v.is_finite());
        Self(v)
    }

    pub fn sub_clamped(self, delta: f32) -> Self {
        let v = (self.0 - delta).max(0.0);
        debug_assert!(v.is_finite());
        Self(v)
    }

    pub fn is_alive(self) -> bool {
        self.0 > 0.0
    }
}

/// Radius in world units.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Radius(pub f32);

/// Speed in meters per second.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct MetersPerSec(pub f32);

/// Duration in seconds. Always >= 0.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Seconds(pub f32);

impl Seconds {
    pub fn new(v: f32) -> Self {
        Self(v.max(0.0))
    }

    /// Decrement by dt, clamped to 0.
    pub fn dec(self, dt: f32) -> Self {
        Self((self.0 - dt).max(0.0))
    }

    pub fn is_expired(self) -> bool {
        self.0 <= 0.0
    }
}

/// Multiplier value. Clamped to [0, MAX_MULT].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Multiplier(pub f32);

impl Multiplier {
    pub const MAX: f32 = 10.0;

    pub fn new(v: f32) -> Self {
        debug_assert!(v.is_finite(), "Multiplier must be finite");
        Self(v.clamp(0.0, Self::MAX))
    }

    pub fn one() -> Self {
        Self(1.0)
    }
}

impl Default for Multiplier {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Mul for Multiplier {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::new(self.0 * rhs.0)
    }
}

/// Angle in radians, normalized to [0, TAU).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct AngleRad(pub f32);

impl AngleRad {
    pub fn new(v: f32) -> Self {
        Self(v.rem_euclid(TAU))
    }

    pub fn advance(self, delta: f32) -> Self {
        Self::new(self.0 + delta)
    }
}

/// Discrete tick counter (u64, checked arithmetic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Tick(pub u64);

impl Tick {
    pub fn next(self) -> Self {
        Self(self.0.checked_add(1).expect("tick overflow"))
    }
}

// ── Enums ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartSlot {
    WeaponWheel,
    Shaft,
    Chassis,
    TraitScrew,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AimMode {
    FollowSpin,
    SeekNearestTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponKind {
    Melee,
    Ranged,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ControlEffect {
    Stun { duration: Seconds },
    Slow { duration: Seconds, ratio: f32 },
    Knockback { distance: f32 },
}

impl ControlEffect {
    /// Apply control reduction multiplier `m` to this effect.
    pub fn apply_reduction(self, m: f32) -> Self {
        let m = m.max(0.0);
        match self {
            Self::Stun { duration } => Self::Stun {
                duration: Seconds::new(duration.0 * m),
            },
            Self::Slow { duration, ratio } => Self::Slow {
                duration: Seconds::new(duration.0 * m),
                ratio,
            },
            Self::Knockback { distance } => Self::Knockback {
                distance: distance * m,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageKind {
    Collision,
    Melee,
    Projectile,
    Wall,
    Obstacle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CollisionBehavior {
    Solid,
    DamageOnHit,
    ApplyControlOnHit,
}
