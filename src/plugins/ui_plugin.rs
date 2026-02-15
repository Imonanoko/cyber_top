use bevy::prelude::*;

use crate::config::tuning::Tuning;
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
    commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0), // 行距
            padding: UiRect::all(Val::Px(0.0)),
            ..default()
        },))
        .with_children(|parent| {
            // HP display line
            parent.spawn((
                HpText,
                Text::new("HP: ---"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Phase / instructions line
            parent.spawn((
                PhaseText,
                Text::new("Phase: Aiming"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.2)),
            ));
        });
}

fn update_hp_display(
    tuning: Res<Tuning>,
    player_hp_q: Query<&SpinHpCurrent, With<PlayerControlled>>,
    ai_hp_q: Query<&SpinHpCurrent, (With<AiControlled>, Without<PlayerControlled>)>,

    // NEW: velocity queries
    player_vel_q: Query<&Velocity, With<PlayerControlled>>,
    ai_vel_q: Query<&Velocity, (With<AiControlled>, Without<PlayerControlled>)>,

    mut text_query: Query<&mut Text, With<HpText>>,
) {
    let player_hp = player_hp_q.iter().next().map(|s| s.0.0).unwrap_or(0.0);
    let ai_hp = ai_hp_q.iter().next().map(|s| s.0.0).unwrap_or(0.0);

    let player_v = player_vel_q
        .iter()
        .next()
        .map(|v| v.0.length())
        .unwrap_or(0.0);
    let ai_v = ai_vel_q.iter().next().map(|v| v.0.length()).unwrap_or(0.0);

    for mut text in &mut text_query {
        **text = format!(
            "Player HP: {:.1} | AI HP: {:.1} | Player v: {:.2} | AI v: {:.2}",
            player_hp, ai_hp, player_v, ai_v
        );
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
    let player_hp = player.iter().next().map(|s| s.0.0).unwrap_or(0.0);
    let ai_hp = ai.iter().next().map(|s| s.0.0).unwrap_or(0.0);

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
