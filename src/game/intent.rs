use bevy::prelude::*;

/// Input intent: written in Update, consumed in FixedUpdate.
#[derive(Component, Default)]
pub struct Intent {
    /// Desired movement direction (normalized or zero).
    pub move_dir: Vec2,
    /// Whether the player wants to fire their weapon.
    pub fire: bool,
}
