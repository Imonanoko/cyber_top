use bevy::prelude::*;

use super::stats::types::{ControlEffect, DamageKind};

/// Top–Top collision event (separate message type to avoid Res/ResMut conflict).
#[derive(Message, Debug, Clone)]
pub struct CollisionMessage {
    pub a: Entity,
    pub b: Entity,
    pub impulse: f32,
    pub normal: Vec2,
}

/// All game events processed through the event pipeline.
#[derive(Message, Debug, Clone)]
pub enum GameEvent {
    DealDamage {
        src: Option<Entity>,
        dst: Entity,
        amount: f32,
        kind: DamageKind,
    },
    ApplyControl {
        dst: Entity,
        control: ControlEffect,
    },
    SpawnProjectile {
        src: Entity,
        position: Vec2,
        direction: Vec2,
        speed: f32,
        damage: f32,
        radius: f32,
        lifetime: f32,
        weapon_id: String,
        /// Sprite visual length (world units). For rectangular projectiles like arrows.
        visual_len: f32,
        /// Sprite visual thickness (world units).
        visual_thick: f32,
    },
    DespawnEntity {
        entity: Entity,
    },
}
