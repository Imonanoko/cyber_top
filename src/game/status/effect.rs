use crate::game::stats::types::Seconds;

/// A status effect definition.
#[derive(Debug, Clone)]
pub struct StatusEffectDef {
    pub name: String,
    pub duration: Seconds,
    pub magnitude: f32,
    pub kind: StatusEffectType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusEffectType {
    DamageOverTime,
    SpeedBuff,
    SpeedDebuff,
}
