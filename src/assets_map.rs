use bevy::prelude::*;
use std::collections::HashMap;

/// Centralized asset handles for all game visuals and audio.
/// Loaded at startup, read-only during gameplay.
#[derive(Resource)]
pub struct GameAssets {
    /// Wheel ID → sprite handle. Missing entries → fallback to procedural mesh.
    pub wheel_sprites: HashMap<String, Handle<Image>>,
    /// Weapon ID → sprite handle.
    pub weapon_sprites: HashMap<String, Handle<Image>>,
    /// Weapon ID → projectile sprite handle (for ranged weapons).
    pub projectile_sprites: HashMap<String, Handle<Image>>,
    /// Fallback colors when sprites are missing.
    pub fallback_colors: HashMap<String, Color>,
    /// Aim arrow sprite (shared between P1 and P2, tinted at spawn time).
    pub aim_arrow: Handle<Image>,
    /// Sound effect handles.
    pub sfx: SfxHandles,
}

/// All sound effect handles, loaded at startup.
pub struct SfxHandles {
    pub launch: Handle<AudioSource>,
    pub collision_top: Handle<AudioSource>,
    pub collision_wall: Handle<AudioSource>,
    pub melee_hit: Handle<AudioSource>,
    pub ranged_fire: Handle<AudioSource>,
    pub projectile_hit: Handle<AudioSource>,
}

impl GameAssets {
    pub fn wheel_sprite(&self, wheel_id: &str) -> Option<&Handle<Image>> {
        self.wheel_sprites.get(wheel_id)
    }

    pub fn weapon_sprite(&self, weapon_id: &str) -> Option<&Handle<Image>> {
        self.weapon_sprites.get(weapon_id)
    }

    pub fn fallback_color(&self, id: &str) -> Color {
        self.fallback_colors
            .get(id)
            .copied()
            .unwrap_or(Color::srgb(0.5, 0.5, 0.5))
    }
}
