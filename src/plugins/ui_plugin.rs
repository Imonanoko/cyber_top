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
    player_q: Query<
        (&SpinHpCurrent, &Velocity, &SpeedBoostEffect, &DamageBoostActive, &TopBuild, &TopEffectiveStats),
        With<PlayerControlled>,
    >,
    ai_q: Query<
        (&SpinHpCurrent, &Velocity, &SpeedBoostEffect, &DamageBoostActive, &TopBuild, &TopEffectiveStats),
        (With<AiControlled>, Without<PlayerControlled>, Without<Player2Controlled>),
    >,
    p2_q: Query<
        (&SpinHpCurrent, &Velocity, &SpeedBoostEffect, &DamageBoostActive, &TopBuild, &TopEffectiveStats),
        (With<Player2Controlled>, Without<PlayerControlled>, Without<AiControlled>),
    >,
    mut text_query: Query<&mut Text, With<HpText>>,
) {
    struct TopInfo {
        name: String,
        hp: f32,
        eff_speed: f32,
        wpn_dmg: f32,
    }

    let extract = |(hp, vel, spd, dmg, build, stats): (
        &SpinHpCurrent, &Velocity, &SpeedBoostEffect, &DamageBoostActive, &TopBuild, &TopEffectiveStats,
    )| {
        let base_wpn_dmg = if let Some(melee) = &build.0.weapon.melee {
            melee.base_damage
        } else if let Some(ranged) = &build.0.weapon.ranged {
            ranged.projectile_damage
        } else {
            0.0
        };
        TopInfo {
            name: build.0.name.clone(),
            hp: hp.0.0,
            eff_speed: vel.0.length() * spd.multiplier,
            wpn_dmg: base_wpn_dmg * stats.0.damage_out_mult.0 * dmg.multiplier,
        }
    };

    let default_info = TopInfo { name: "???".into(), hp: 0.0, eff_speed: 0.0, wpn_dmg: 0.0 };

    let p1 = player_q.iter().next().map(extract)
        .unwrap_or(TopInfo { name: "???".into(), hp: 0.0, eff_speed: 0.0, wpn_dmg: 0.0 });
    let p2 = ai_q.iter().next()
        .or_else(|| p2_q.iter().next())
        .map(extract)
        .unwrap_or(default_info);

    for mut text in &mut text_query {
        **text = format!(
            "{}  HP:{:.1}  spd:{:.1}  wpn:{:.1}\n{}  HP:{:.1}  spd:{:.1}  wpn:{:.1}",
            p1.name, p1.hp, p1.eff_speed, p1.wpn_dmg,
            p2.name, p2.hp, p2.eff_speed, p2.wpn_dmg,
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
