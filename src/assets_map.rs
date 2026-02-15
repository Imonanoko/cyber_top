use bevy::prelude::*;
use std::collections::HashMap;

/// Maps skin_id to visual properties for placeholder rendering.
#[derive(Resource, Default)]
pub struct AssetsMap {
    pub skin_colors: HashMap<String, Color>,
}

impl AssetsMap {
    pub fn with_defaults() -> Self {
        let mut map = HashMap::new();
        map.insert("default".into(), Color::srgb(0.2, 0.6, 1.0));
        map.insert("red_spinner".into(), Color::srgb(1.0, 0.2, 0.2));
        map.insert("green_spinner".into(), Color::srgb(0.2, 1.0, 0.3));
        map.insert("gold_spinner".into(), Color::srgb(1.0, 0.85, 0.0));
        Self { skin_colors: map }
    }

    pub fn get_color(&self, skin_id: &str) -> Color {
        self.skin_colors
            .get(skin_id)
            .copied()
            .unwrap_or(Color::srgb(0.5, 0.5, 0.5))
    }
}
