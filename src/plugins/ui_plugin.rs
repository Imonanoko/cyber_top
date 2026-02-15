use bevy::prelude::*;

use crate::game::components::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui);
        app.add_systems(Update, update_hp_display);
    }
}

#[derive(Component)]
struct HpText;

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        HpText,
        Text::new("HP: ---"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
    ));
}

fn update_hp_display(
    player: Query<&SpinHpCurrent, With<PlayerControlled>>,
    ai: Query<&SpinHpCurrent, (With<AiControlled>, Without<PlayerControlled>)>,
    mut text_query: Query<&mut Text, With<HpText>>,
) {
    let player_hp = player.iter().next().map(|s| s.0 .0).unwrap_or(0.0);
    let ai_hp = ai.iter().next().map(|s| s.0 .0).unwrap_or(0.0);

    for mut text in &mut text_query {
        **text = format!("Player HP: {:.1}  |  AI HP: {:.1}", player_hp, ai_hp);
    }
}
