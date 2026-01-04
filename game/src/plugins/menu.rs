use bevy::prelude::*;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::keyboard::*;
use bevy::color::palettes::css::*;

use crate::prelude::*;
use crate::prelude::{vjoy_base::VjoyBase, dash::DashButton,};
use crate::resources::leaderboard::*;
use crate::resources::score::*;
use crate::resources::player_profile::*;
use crate::resources::session_time::*;
use crate::resources::reset_target::*;
use crate::resources::leaderboard_channel::*;
use crate::resources::firebase_config::*;
use crate::components::ui::*;

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<Leaderboard>()
        .init_resource::<LeaderboardChannel>()
        .init_resource::<FirebaseConfig>()
        .init_resource::<Leaderboard>()
        .register_type::<Leaderboard>()
        .add_systems(OnEnter(GameState::MainMenu), (setup_main_menu, trigger_leaderboard_fetch))
        .add_systems(Update, (ui_button_hover_system, (main_menu_system, leaderboard_scroll_system, username_typing_system, toggle_ime_system, leaderboard_receiver_system, update_leaderboard_ui_system).run_if(in_state(GameState::MainMenu))))
        .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
        .add_systems(Update, (
            (crate::plugins::vjoy::sync_dash_text_size)
        ).run_if(in_state(GameState::Playing)))
        .add_systems(OnEnter(GameState::GameOver), (setup_death_menu, cleanup_game_ui))
        .add_systems(Update, death_menu_interaction_system.run_if(in_state(GameState::GameOver)))
        .add_systems(OnExit(GameState::GameOver), cleanup_death_menu);
}

fn setup_death_menu(
    mut commands: Commands, 
    score: Res<Score>, 
    time: Res<SessionTime>,
    profile: Res<PlayerProfile>
) {
    commands.spawn((
        DeathMenuRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::VMin(2.0), 
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
        ZIndex(200),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("GAME OVER!"),
            TextFont { font_size: 80.0, ..default() },
            TextColor(Color::srgb(1.0, 0.1, 0.1)),
        ));

        parent.spawn((
            Text::new(format!("SCORE: {} | TIME: {}", score.current, time.format())),
            TextFont { font_size: 30.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::VMin(2.0)), ..default() },
        ));

        if !profile.username.trim().is_empty() {
            spawn_menu_button(parent, SubmitScoreButton, "SUBMIT TO CLOUD", Color::srgb(0.0, 0.6, 0.8));
        }

        spawn_menu_button(parent, RestartButton, "RESTART", Color::srgb(0.2, 0.2, 0.2));
        spawn_menu_button(parent, MainMenuButton, "MAIN MENU", Color::srgb(0.1, 0.3, 0.1));
    });
}

fn death_menu_interaction_system(
    mut next_state: ResMut<NextState<GameState>>,
    mut reset_target: ResMut<ResetTarget>,
    mut leaderboard: ResMut<Leaderboard>,
    score: Res<Score>,
    time: Res<SessionTime>,
    mut profile: ResMut<PlayerProfile>,
    config: Res<FirebaseConfig>,
    q_restart: Query<&Interaction, (Changed<Interaction>, With<RestartButton>)>,
    q_menu: Query<&Interaction, (Changed<Interaction>, With<MainMenuButton>)>,
    q_submit: Query<(Entity, &Interaction), (Changed<Interaction>, With<SubmitScoreButton>)>,
    mut commands: Commands,
) {
    if let Ok(Interaction::Pressed) = q_restart.single() {
        reset_target.0 = GameState::Playing;
        next_state.set(GameState::Resetting);
    }

    if let Ok(Interaction::Pressed) = q_menu.single() {
        reset_target.0 = GameState::MainMenu;
        next_state.set(GameState::Resetting);
    }

    if let Ok((btn_entity, Interaction::Pressed)) = q_submit.single() {
        let entry = FirebaseEntry {
            name: profile.username.clone(),
            score: score.current,
            time: time.elapsed,
        };

        let url = format!("{}leaderboard.json", config.url);
        let json = serde_json::to_string(&entry).unwrap();
        let request = ehttp::Request::post(url, json.into_bytes());

        ehttp::fetch(request, |result| {
            if let Ok(resp) = result {
                if resp.status == 200 { println!("Score accepted by Firebase!"); }
            }
        });

        leaderboard.entries.push((profile.username.clone(), score.current, time.elapsed));
        leaderboard.entries.sort_by(|a, b| b.1.cmp(&a.1));
        
        profile.username = String::new();
        if let Ok(mut entity_cmds) = commands.get_entity(btn_entity) {
            entity_cmds.despawn();
        }
    }
}

fn cleanup_death_menu(mut commands: Commands, q_root: Query<Entity, With<DeathMenuRoot>>) {
    if let Ok(entity) = q_root.single() {
        commands.entity(entity).despawn_children();
        commands.entity(entity).despawn();
    }
}

fn setup_main_menu(mut commands: Commands, leaderboard: Res<Leaderboard>) {
    commands.spawn((
        MainMenuRoot, 
        SessionUi,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ZIndex(200),
    ))
    .with_children(|parent| {
        
        parent.spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(15.0),
            ..default()
        })
        .with_children(|menu| {
            menu.spawn((
                Text::new("GREEN SPHERE"),
                TextFont { font_size: 80.0, ..default() },
                TextColor(Color::srgb(0.0, 1.0, 0.5))
            ));
            
            menu.spawn((
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderRadius::all(Val::Px(5.0)),
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 1.0)),
            ))
            .with_children(|p| {
                p.spawn((
                    UsernameInputText,
                    Text::new("TYPE NAME..."),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::from(GRAY))
                ));
            });

            spawn_menu_button(menu, StartButton, "START MISSION", Color::srgb(0.2, 0.2, 0.2));
            spawn_menu_button(menu, ShowLeaderboardButton, "LEADERBOARD", Color::srgb(0.2, 0.2, 0.4));
            spawn_menu_button(menu, ExitButton, "EXIT", Color::srgb(0.2, 0.1, 0.1));
        });

        parent.spawn((
            LeaderboardPanel,
            Interaction::default(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.98)),
            Visibility::Hidden,
        ))
        .with_children(|overlay| {
            overlay.spawn((
                Node {
                    width: Val::Px(600.0),
                    height: Val::Px(550.0),
                    padding: UiRect::all(Val::Px(30.0)),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                BorderRadius::all(Val::Px(15.0))
            ))
            .with_children(|box_node| {
                box_node.spawn((
                    Text::new("TOP SURVIVORS"),
                    TextFont { font_size: 32.0, ..default() },
                    TextColor(Color::from(YELLOW)),
                    Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() }
                ));

                box_node.spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(300.0),
                    overflow: Overflow::clip_y(),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|viewport| {
                    viewport.spawn((
                        LeaderboardContentArea,
                        LeaderboardList,
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            width: Val::Percent(100.0),
                            position_type: PositionType::Relative,
                            ..default()
                        },
                    ))
                    .with_children(|list| {
                        for (name, score, time) in &leaderboard.entries {
                            spawn_leaderboard_row(list, name, *score, *time);
                        }
                    });
                });

                spawn_menu_button(box_node, CloseLeaderboardButton, "BACK", Color::srgb(0.3, 0.3, 0.3));
            });
        });
    });
}

fn spawn_leaderboard_row(parent: &mut ChildSpawnerCommands, name: &str, score: usize, time: f32) {
    parent.spawn((
        Node {
            display: Display::Flex,
            justify_content: JustifyContent::SpaceBetween,
            width: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::bottom(Val::Px(1.0)),
            ..default()
        },
        BorderColor::from(BLACK),
    ))
    .with_children(|row| {
        row.spawn((Text::new(name.to_string()), TextColor(Color::WHITE)));
        
        let mins = (time / 60.0) as u32; 
        let secs = (time % 60.0) as u32;
        row.spawn((
            Text::new(format!("{} [{:02}:{:02}]", score, mins, secs)), 
            TextColor(Color::from(LIGHT_CYAN))
        ));
    });
}

fn main_menu_system(
    mut next_state: ResMut<NextState<GameState>>,
    q_start: Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    q_exit: Query<&Interaction, (Changed<Interaction>, With<ExitButton>)>,
    q_lb_show: Query<&Interaction, (Changed<Interaction>, With<ShowLeaderboardButton>)>,
    q_lb_close: Query<&Interaction, (Changed<Interaction>, With<CloseLeaderboardButton>)>,
    mut q_panel: Query<&mut Visibility, With<LeaderboardPanel>>,
    mut exit_events: MessageWriter<AppExit>,
) {
    let Ok(panel_visibility) = q_panel.single() else { return; };
    
    let is_leaderboard_open = *panel_visibility != Visibility::Hidden;

    if !is_leaderboard_open {
        if let Ok(Interaction::Pressed) = q_start.single() {
            next_state.set(GameState::Playing);
        }

        if let Ok(Interaction::Pressed) = q_lb_show.single() {
            if let Ok(mut vis) = q_panel.single_mut() {
                *vis = Visibility::Inherited;
            }
        }

        if let Ok(Interaction::Pressed) = q_exit.single() {
            exit_events.write(AppExit::Success);
        }
    }

    if is_leaderboard_open {
        if let Ok(Interaction::Pressed) = q_lb_close.single() {
            if let Ok(mut vis) = q_panel.single_mut() {
                *vis = Visibility::Hidden;
            }
        }
    }
}

fn cleanup_main_menu(mut commands: Commands, q: Query<Entity, With<MainMenuRoot>>) {
    if let Ok(e) = q.single() {
        commands.entity(e).despawn();
    }
}

fn ui_button_hover_system(
    mut q_buttons: Query<(&Interaction, &mut BackgroundColor), (With<Button>, Changed<Interaction>)>,
) {
    for (interaction, mut bg) in q_buttons.iter_mut() {
        match *interaction {
            Interaction::Hovered => bg.0 = Color::srgb(0.3, 0.3, 0.3),
            Interaction::None => bg.0 = Color::srgb(0.2, 0.2, 0.2),
            _ => {}
        }
    }
}

fn spawn_menu_button<T: Component>(
    parent: &mut ChildSpawnerCommands, 
    marker: T, 
    label: &str, 
    color: Color
) {
    parent.spawn((
        Button,
        marker,
        Interaction::default(),
        Node {
            width: Val::Px(250.0),
            height: Val::Px(60.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(color),
        BorderRadius::all(Val::Px(10.0)),
    ))
    .with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont { font_size: 25.0, ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn leaderboard_scroll_system(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    touches: Res<Touches>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    q_viewport: Query<(&Node, &GlobalTransform, &Interaction), With<LeaderboardPanel>>, 
    mut q_list: Query<(&mut Node, &ComputedNode), (With<LeaderboardList>, Without<LeaderboardPanel>)>,
    mut last_drag_pos: Local<Option<Vec2>>,
) {
    let Ok((mut list_node, list_computed)) = q_list.single_mut() else { return; };
    let Ok((_viewport_node, _viewport_transform, interaction)) = q_viewport.single() else { return; };
    let Ok(window) = q_window.single() else { return; };

    let mut scroll_delta = 0.0;

    for event in mouse_wheel_events.read() {
        scroll_delta += match event.unit {
            MouseScrollUnit::Line => event.y * 20.0,
            MouseScrollUnit::Pixel => event.y,
        };
    }

    let current_pos = if touches.any_just_pressed() || touches.any_just_pressed() {
        touches.first_pressed_position()
    } else if mouse_buttons.pressed(MouseButton::Left) {
        window.cursor_position()
    } else {
        None
    };

    if let Some(pos) = current_pos {
        if *interaction != Interaction::None {
            if let Some(last_pos) = *last_drag_pos {
                scroll_delta += pos.y - last_pos.y;
            }
            *last_drag_pos = Some(pos);
        }
    } else {
        *last_drag_pos = None;
    }

    if scroll_delta != 0.0 {
        let mut current_top = if let Val::Px(t) = list_node.top { t } else { 0.0 };
        current_top += scroll_delta;

        let list_height = list_computed.size().y;
        let max_scroll = (list_height - 300.0).max(0.0);
        
        current_top = current_top.clamp(-max_scroll, 0.0);
        list_node.top = Val::Px(current_top);
    }
}

fn username_typing_system(
    mut key_msgs: MessageReader<KeyboardInput>,
    mut ime_msgs: MessageReader<Ime>,
    mut profile: ResMut<PlayerProfile>,
    mut q_text: Query<&mut Text, With<UsernameInputText>>,
) {
    let mut changed = false;

    for event in key_msgs.read() {
        if !event.state.is_pressed() { continue; }

        match &event.logical_key {
            Key::Backspace => {
                profile.username.pop();
                changed = true;
            }
            Key::Character(smol_str) => {
                for c in smol_str.chars() {
                    if !c.is_control() && profile.username.len() < 12 {
                        profile.username.push(c);
                        changed = true;
                    }
                }
            }
            _ => {}
        }
    }

    for msg in ime_msgs.read() {
        if let Ime::Commit { value, .. } = msg {
            for c in value.chars() {
                if !c.is_control() && profile.username.len() < 12 {
                    profile.username.push(c);
                    changed = true;
                }
            }
        }
    }

    if changed {
        if let Ok(mut text) = q_text.single_mut() {
            text.0 = if profile.username.is_empty() {
                "TYPE NAME...".to_string()
            } else {
                profile.username.clone()
            };
        }
    }
}

fn toggle_ime_system(
    q_input_box: Query<&Interaction, (With<UsernameInputText>, Changed<Interaction>)>,
    mut q_window: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    let Ok(interaction) = q_input_box.single() else { return; };
    let Ok(mut window) = q_window.single_mut() else { return; };

    if *interaction == Interaction::Pressed {
        window.ime_enabled = true;
        
        window.ime_position = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        
    }
}

fn trigger_leaderboard_fetch(config: Res<FirebaseConfig>, channel: Res<LeaderboardChannel>) {
    let tx = channel.tx.clone();
    let url = format!("{}leaderboard.json?orderBy=\"score\"&limitToLast=10", config.url);
    
    let request = ehttp::Request::get(url);

    ehttp::fetch(request, move |result| {
        match result {
            Ok(response) => {
                if response.status == 200 {
                    let raw_json: serde_json::Value = serde_json::from_slice(&response.bytes).unwrap_or_default();
                    
                    if raw_json.is_null() {
                        println!("FIREBASE: Database is empty.");
                        return;
                    }

                    if let Some(obj) = raw_json.as_object() {
                        let mut entries = Vec::new();
                        for (_id, val) in obj {
                            if let (Some(name), Some(score), Some(time)) = (
                                val.get("name").and_then(|v| v.as_str()),
                                val.get("score").and_then(|v| v.as_u64()),
                                val.get("time").and_then(|v| v.as_f64()),
                            ) {
                                entries.push((name.to_string(), score as usize, time as f32));
                            }
                        }

                        entries.sort_by(|a, b| b.1.cmp(&a.1));
                        
                        let _ = tx.send(entries);
                        println!("FIREBASE: Successfully parsed {} entries", obj.len());
                    }
                } else {
                    println!("FIREBASE FETCH ERROR: Status {}", response.status);
                }
            }
            Err(e) => println!("NETWORK ERROR: {}", e),
        }
    });
}

fn leaderboard_receiver_system(
    channel: Res<LeaderboardChannel>,
    mut leaderboard: ResMut<Leaderboard>,
) {
    let new_data = if let Ok(rx) = channel.rx.lock() {
        rx.try_recv().ok()
    } else {
        None
    };

    if let Some(new_entries) = new_data {
        leaderboard.entries = new_entries;
        println!("LOCAL: Leaderboard resource updated with fresh data!");
    }
}

fn update_leaderboard_ui_system(
    mut commands: Commands,
    leaderboard: Res<Leaderboard>,
    q_container: Query<Entity, With<LeaderboardContentArea>>,
) {
    if !leaderboard.is_changed() { return; }
    let Ok(container_entity) = q_container.single() else { return; };

    commands.entity(container_entity).despawn_children();

    commands.entity(container_entity).with_children(|parent| {
        if leaderboard.entries.is_empty() {
            parent.spawn((
                Text::new("NO SCORES YET..."),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::from(bevy::color::palettes::css::GRAY)),
            ));
        }

        for (name, score, time_val) in &leaderboard.entries {
            parent.spawn((
                Node {
                    display: Display::Flex,
                    justify_content: JustifyContent::SpaceBetween,
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::from(bevy::color::palettes::css::BLACK),
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new(name.clone()), 
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(Color::WHITE)
                ));
                
                let mins = (*time_val / 60.0) as u32;
                let secs = (*time_val % 60.0) as u32;
                row.spawn((
                    Text::new(format!("{} [{:02}:{:02}]", score, mins, secs)), 
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(Color::from(bevy::color::palettes::css::LIGHT_CYAN))
                ));
            });
        }
    });
}

fn cleanup_game_ui(
    mut commands: Commands,
    q_ui: Query<Entity, Or<(With<VjoyBase>, With<DashButton>, With<HealthBarFill>, With<ScoreHud>, With<DeathMenuRoot>, With<SessionUi>)>>,
) {
    for entity in q_ui.iter() {
        if let Ok(mut entity_cmds) = commands.get_entity(entity) {
            entity_cmds.despawn();
        }
    }
}