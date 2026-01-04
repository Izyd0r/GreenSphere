use bevy::prelude::*;

use crate::prelude::*;
use crate::prelude::ui::*;
use crate::prelude::notification_timer::*;
use crate::prelude::score::*;
use crate::prelude::session_time::*;
use crate::prelude::player_ball::*;

pub(crate) fn plugin(app: &mut App) {
    app
        .add_systems(OnEnter(GameState::Playing), (
            crate::plugins::vjoy::spawn_joystick,    
            crate::plugins::vjoy::spawn_dash_button,
            spawn_health_bar,
            spawn_score_hud,
            spawn_factory_notification,
        ).chain())        
        .add_systems(Update, (
            (score_event_handler, update_score_hud_system),
            (track_session_time_system, update_time_hud_system),
            (notification_lifecycle_system),
            (crate::plugins::vjoy::sync_dash_text_size)
        ).run_if(in_state(GameState::Playing)).run_if(any_with_component::<PlayerBall>));
}

fn notification_lifecycle_system(
    time: Res<Time>,
    mut q_parent: Query<(&mut Visibility, &mut NotificationTimer, &Children), With<FactoryNotificationText>>,
    mut q_text: Query<&mut TextColor>,
) {
    let Ok((mut vis, mut timer, children)) = q_parent.single_mut() else { return; };

    if *vis == Visibility::Hidden { return; }

    timer.0.tick(time.delta());

    if timer.0.just_finished() {
        *vis = Visibility::Hidden;
    } else {
        let progress = timer.0.fraction_remaining();
        for &child in children { 
            if let Ok(mut color) = q_text.get_mut(child) {
                color.0 = color.0.with_alpha(progress);
            }
        }
    }
}

fn spawn_factory_notification(mut commands: Commands) {
    commands.spawn((
        SessionUi,
        FactoryNotificationText,
        NotificationTimer(Timer::from_seconds(3.0, TimerMode::Once)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::VMin(12.0), 
            width: Val::Percent(100.0),
            display: Display::Flex,
            justify_content: JustifyContent::Center, 
            align_items: AlignItems::Center,
            ..default()
        },
        Visibility::Hidden,
        ZIndex(100),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("! FACTORY DEPLOYED !"),
            TextFont { font_size: 30.0, ..default() },
            TextColor(Color::srgb(1.0, 0.0, 0.0)), 
        ));
    });
}

fn score_event_handler(
    mut messages: MessageReader<ScoreMessage>,
    mut score: ResMut<Score>,
) {
    for msg in messages.read() {
        score.current += msg.0;
    }
}

fn spawn_score_hud(mut commands: Commands) {
    commands.spawn((
        ScoreHud, 
        SessionUi,
        Node {
            position_type: PositionType::Absolute,
            top: Val::VMin(2.0),
            width: Val::Percent(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::VMin(10.0),
            ..default()
        },
        ZIndex(100),
    ))
    .with_children(|parent| {
        parent.spawn((
            ScoreHudText,
            Text::new("SCORE: 0"),
            TextFont { font_size: 24.0, ..default() },
            TextColor(Color::WHITE),
        ));

        parent.spawn((
            TimeHudText,
            Text::new("TIME: 00:00"),
            TextFont { font_size: 24.0, ..default() },
            TextColor(Color::srgb(0.8, 0.8, 1.0)),
        ));
    });
}

fn update_score_hud_system(
    score: Res<Score>,
    mut q_text: Query<&mut Text, With<ScoreHudText>>,
) {
    if score.is_changed() {
        if let Ok(mut text) = q_text.single_mut() {
            text.0 = format!("SCORE: {}", score.current);
        }
    }
}

fn track_session_time_system(
    time: Res<Time>,
    mut session_time: ResMut<SessionTime>,
) {
    session_time.elapsed += time.delta_secs();
}

fn update_time_hud_system(
    session_time: Res<SessionTime>,
    mut q_text: Query<&mut Text, With<TimeHudText>>,
) {
    if let Ok(mut text) = q_text.single_mut() {
        text.0 = format!("TIME: {}", session_time.format());
    }
}

fn spawn_health_bar(mut commands: Commands) {
    commands.spawn((
        SessionUi,
        Node {
            position_type: PositionType::Absolute,
            display: Display::Flex,
            flex_direction: FlexDirection::Column, 
            align_items: AlignItems::Center,
            left: Val::Percent(50.0),
            bottom: Val::VMin(18.0),
            margin: UiRect::left(Val::VMin(-15.0)),
            ..default()
        },
        ZIndex(100),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("HEALTH"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
        ));

        parent.spawn((
            Node {
                width: Val::VMin(30.0),
                height: Val::VMin(2.5),
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|bar| {
            bar.spawn((
                HealthBarFill,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.0, 1.0, 0.0)),
                BorderRadius::all(Val::Px(4.0)),
            ));

            bar.spawn((
                HealthText,
                Text::new("100 / 100"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::WHITE),
                ZIndex(1),
            ));
        });
    });
}