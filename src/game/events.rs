use bevy::prelude::*;

use super::stats::types::{ControlEffect, DamageKind};

/// All game events processed through the event pipeline.
#[derive(Message, Debug, Clone)]
pub enum GameEvent {
    Collision {
        a: Entity,
        b: Entity,
        impulse: f32,
        normal: Vec2,
    },
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
