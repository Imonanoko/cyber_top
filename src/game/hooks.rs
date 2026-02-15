use bevy::prelude::*;

use super::events::GameEvent;

/// Hook pipeline: processes events through part hooks, status hooks, etc.
/// For v0 this is a pass-through; hooks will be added in v0.2.
pub fn process_hooks(
    mut _events: MessageReader<GameEvent>,
    // In future: query for TraitScrew hooks, status effects, floor zones, etc.
) {
    // v0: no-op pass-through
    // v0.2: iterate events, run through trait screw on_hit, on_tick hooks, etc.
}
