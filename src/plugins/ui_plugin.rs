use bevy::prelude::*;

use crate::game::components::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GamePhase::Aiming), setup_ui);
        app.add_systems(
            Update,
            (update_hp_display, update_phase_display)
                .run_if(in_state(GamePhase::Aiming).or(in_state(GamePhase::Battle)).or(in_state(GamePhase::GameOver))),
        );
    }
}

#[derive(Component)]
struct HpText;

#[derive(Component)]
struct PhaseText;

fn setup_ui(mut commands: Commands) {
    commands
        .spawn((
            InGame,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(0.0)),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                HpText,
                Text::new("HP: ---"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

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
    player_hp_q: Query<&SpinHpCurrent, With<PlayerControlled>>,
    ai_hp_q: Query<&SpinHpCurrent, (With<AiControlled>, Without<PlayerControlled>, Without<Player2Controlled>)>,
    p2_hp_q: Query<&SpinHpCurrent, (With<Player2Controlled>, Without<PlayerControlled>, Without<AiControlled>)>,

    player_vel_q: Query<&Velocity, With<PlayerControlled>>,
    ai_vel_q: Query<&Velocity, (With<AiControlled>, Without<PlayerControlled>, Without<Player2Controlled>)>,
    p2_vel_q: Query<&Velocity, (With<Player2Controlled>, Without<PlayerControlled>, Without<AiControlled>)>,

    mut text_query: Query<&mut Text, With<HpText>>,
) {
    let p1_hp = player_hp_q.iter().next().map(|s| s.0 .0).unwrap_or(0.0);
    let p2_hp = ai_hp_q
        .iter()
        .next()
        .or_else(|| p2_hp_q.iter().next())
        .map(|s| s.0 .0)
        .unwrap_or(0.0);

    let p1_v = player_vel_q.iter().next().map(|v| v.0.length()).unwrap_or(0.0);
    let p2_v = ai_vel_q
        .iter()
        .next()
        .or_else(|| p2_vel_q.iter().next())
        .map(|v| v.0.length())
        .unwrap_or(0.0);

    for mut text in &mut text_query {
        **text = format!(
            "P1 HP: {:.1} | P2 HP: {:.1} | P1 v: {:.2} | P2 v: {:.2}",
            p1_hp, p2_hp, p1_v, p2_v
        );
    }
}

fn update_phase_display(
    state: Res<State<GamePhase>>,
    mut text_query: Query<&mut Text, With<PhaseText>>,
) {
    let phase_str = match state.get() {
        GamePhase::Aiming => "Arrows to aim, Space to launch (P2: A/D + Enter)",
        GamePhase::Battle => "Battle!",
        GamePhase::GameOver => "Game Over",
        _ => "",
    };
    for mut text in &mut text_query {
        **text = phase_str.to_string();
    }
}
