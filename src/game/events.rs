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
        tags: Vec<String>,
    },
    ApplyControl {
        src: Option<Entity>,
        dst: Entity,
        control: ControlEffect,
    },
    ApplyStatus {
        src: Option<Entity>,
        dst: Entity,
        status: StatusEffectData,
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
    },
    SpawnObstacle {
        src: Option<Entity>,
        position: Vec2,
        radius: f32,
        ttl: f32,
        behavior: super::stats::types::CollisionBehavior,
    },
    DespawnEntity {
        entity: Entity,
    },
}

/// Data for a status effect instance.
#[derive(Debug, Clone)]
pub struct StatusEffectData {
    pub kind: StatusEffectKind,
    pub duration: f32,
    pub magnitude: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StatusEffectKind {
    DamageOverTime,
    SpeedBuff,
    SpeedDebuff,
}
