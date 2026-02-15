use bevy::prelude::*;

use crate::game::components::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui);
        app.add_systems(Update, (update_hp_display, update_phase_display));
        app.add_systems(OnEnter(GamePhase::GameOver), show_game_over);
    }
}

#[derive(Component)]
struct HpText;

#[derive(Component)]
struct PhaseText;

#[derive(Component)]
struct GameOverText;

fn setup_ui(mut commands: Commands) {
    // HP display
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

    // Phase / instructions display
    commands.spawn((
        PhaseText,
        Text::new("Phase: Aiming"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(40.0),
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

fn update_phase_display(
    state: Res<State<GamePhase>>,
    mut text_query: Query<&mut Text, With<PhaseText>>,
) {
    let phase_str = match state.get() {
        GamePhase::Aiming => "Left/Right to aim, Space to launch",
        GamePhase::Battle => "Battle!",
        GamePhase::GameOver => "Game Over",
    };
    for mut text in &mut text_query {
        **text = phase_str.to_string();
    }
}

fn show_game_over(
    mut commands: Commands,
    player: Query<&SpinHpCurrent, With<PlayerControlled>>,
    ai: Query<&SpinHpCurrent, (With<AiControlled>, Without<PlayerControlled>)>,
) {
    let player_hp = player.iter().next().map(|s| s.0 .0).unwrap_or(0.0);
    let ai_hp = ai.iter().next().map(|s| s.0 .0).unwrap_or(0.0);

    let winner = if player_hp > ai_hp {
        "Player Wins!"
    } else {
        "AI Wins!"
    };

    commands.spawn((
        GameOverText,
        Text::new(winner),
        TextFont {
            font_size: 48.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(35.0),
            top: Val::Percent(40.0),
            ..default()
        },
    ));
}
