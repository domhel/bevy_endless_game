// #![allow(unused)]

// silence unused wanrings while learning
use bevy::sprite::collide_aabb::collide;
use bevy::{prelude::*, render::camera::*, window::*};
use rand::Rng;
// use bevy_kira_audio::{Audio, AudioPlugin, AudioChannel};

const VEL_CLIPPING: f32 = 1e-3;
const PLAYER_SPEED: f32 = 8.0;
const CAMERA_SPEED: f32 = 128.0;

// components
struct Player;
struct Health(f32);
struct Velocity(Vec3);
struct Score(i32);
struct Food;
struct FoodEatenEvent(Entity);
struct GatePassedEvent(Entity);
struct PlayerLostEvent;
struct Wall;
struct Scoreboard;
struct Gate;
struct GestureLine;
struct PauseMenuText;

struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    hovered: Handle<ColorMaterial>,
    pressed: Handle<ColorMaterial>,
}
struct ScoreSound(Handle<AudioSource>);
struct DeathSound(Handle<AudioSource>);


// Resources
struct WindowSize {
    width: f32,
    height: f32,
}
struct WindowSizeDiagonalWeighted(f32);
struct LastWallSpawnedAt(f64);
struct DragGesture {
    start_pos: Vec2,
    is_dragging: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    Playing,
    Dead,
    Paused
}


fn setup(
    mut commands: Commands,
    mut windows: ResMut<Windows>,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands.insert_resource(DragGesture {
        start_pos: Vec2::ZERO,
        is_dragging: false,
    });

    let window = windows.get_primary_mut().unwrap();
    let width = window.width();
    let height = window.height();
    let window_size_diagonal_weighted: f32 = ((width * width + height * height) / 2.0).sqrt();
    // save window size
    commands.insert_resource(WindowSize {
        width: width,
        height: height,
    });
    commands.insert_resource(WindowSizeDiagonalWeighted(window_size_diagonal_weighted));
    // println!("Diagonal: {}", window_size_diagonal_weighted);
    // position window
    // todo: get actual monitor size
    // window.set_position(IVec2::new(
    //     (1920 - width as i32) / 2,
    //     (1080 - height as i32) / 2,
    // ));

    commands.insert_resource(LastWallSpawnedAt(time.seconds_since_startup()));

    // spawn player
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgb_u8(235, 107, 52).into()),
            sprite: Sprite::new(Vec2::new(
                0.03 * window_size_diagonal_weighted,
                0.03 * window_size_diagonal_weighted,
            )),
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 1.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Player)
        .insert(Health(0.0))
        .insert(Score(0))
        .insert(Velocity(Vec3::ZERO));

    // spawn food
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgb_u8(52, 235, 140).into()),
            sprite: Sprite::new(Vec2::new(
                0.025 * window_size_diagonal_weighted,
                0.025 * window_size_diagonal_weighted,
            )),
            transform: Transform {
                translation: Vec3::new(width / 4.0, height / 4.0, 0.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Food);

    // spawn_scoreboard(commands, window_size_diagonal_weighted, asset_server);
    // spawn scoreboard
    commands
        .spawn_bundle(Text2dBundle {
            text: Text {
                alignment: TextAlignment {
                    horizontal: HorizontalAlign::Center,
                    vertical: VerticalAlign::Top,
                },
                sections: vec![TextSection {
                    value: "Score".to_string(),
                    style: TextStyle {
                        font_size: window_size_diagonal_weighted * 0.08,
                        font: asset_server.load("fonts/BaiJamjuree-Medium.ttf"),
                        color: Color::rgb(1.0, 1.0, 1.0),
                    },
                }],
            },
            transform: Transform {
                translation: Vec3::new(0., height / 2.5, 2.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Scoreboard);

    // set button materials
    commands.insert_resource(ButtonMaterials {
        normal: materials.add(Color::rgba_u8(0x37, 0x41, 0x51, 255).into()),
        hovered: materials.add(Color::rgba(0.0, 0.0, 0.0, 0.25).into()),
        pressed: materials.add(Color::rgba_u8(0x37, 0x41, 0x51, 10).into()),
    });

    // score sound: pickupCoin
    commands.insert_resource(ScoreSound(
        asset_server.load("sounds/pickupCoin.wav").into(),
    ));
    commands.insert_resource(DeathSound(asset_server.load("sounds/death.wav").into()));
}

// needs to be called every time because the camera always changes!
fn update_scoreboard(
    mut query_set: QuerySet<(
        Query<(&mut Transform, &mut Text, With<Scoreboard>)>,
        Query<(&Transform, With<OrthographicProjection>)>,
        Query<&Score, With<Player>>,
    )>,
    window_size: Res<WindowSize>,
) {
    let mut score = 0;
    if let Ok(s) = query_set.q2_mut().single_mut() {
        score = s.0;
    }
    let mut camera_y = 0.0f32;
    if let Ok((tf, _)) = query_set.q1_mut().single_mut() {
        camera_y = tf.translation.y;
    }
    if let Ok((mut tf, mut text, _)) = query_set.q0_mut().single_mut() {
        if let Some(section) = text.sections.get_mut(0) {
            section.value = format!("Score {}", score).to_string();
        }
        tf.translation.y = camera_y - window_size.height / 2.5;
    }
}

fn player_check_food(
    mut query_set: QuerySet<(
        Query<(&Transform, &Sprite, With<Player>)>,
        Query<(Entity, &Transform, &Sprite, With<Food>)>,
    )>,
    mut ev_food_eaten: EventWriter<FoodEatenEvent>,
) {
    let mut player_translation: Vec3 = Vec3::ZERO;
    let mut food_translation: Vec3 = Vec3::new(50., 50., 50.);
    let mut player_size = Vec2::ZERO;
    if let Ok((player_tf, sprite, _)) = query_set.q0_mut().single_mut() {
        player_translation = player_tf.translation;
        player_size = sprite.size;
    }
    for (food_entity, food_tf, sprite, __) in query_set.q1_mut().iter_mut() {
        food_translation = food_tf.translation;
        if (player_translation.x - food_translation.x).abs() < (player_size.x + sprite.size.x) / 2.0
            && (player_translation.y - food_translation.y).abs()
                < (player_size.y + sprite.size.y) / 2.0
        {
            ev_food_eaten.send(FoodEatenEvent(food_entity));
            break;
        }
    }
    // for ((mut transform, entity_type)) in query.iter_mut() {
    //     if entity_type = Player {

    //     }
    // }
}

fn food_eaten(
    mut ev_food_eaten: EventReader<FoodEatenEvent>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query_set: QuerySet<(
        Query<(&mut Score, With<Player>)>,
        Query<(&Transform, With<OrthographicProjection>)>,
    )>,
    mut commands: Commands,
    window_size: Res<WindowSize>,
    window_size_diag: Res<WindowSizeDiagonalWeighted>,
) {
    let mut camera_y = 0.0;
    if let Ok((camera_tf, _)) = query_set.q1_mut().single_mut() {
        camera_y = camera_tf.translation.y;
    }
    for ev in ev_food_eaten.iter() {
        commands.entity(ev.0).despawn();
        if let Ok((mut player_score, _)) = query_set.q0_mut().single_mut() {
            player_score.0 += 1;
            // println!("Score: {}", player_score.0);

            // update UI
        }
        let mut rng = rand::thread_rng();
        // duplicate food
        for _i in 0..2 {
            commands
                .spawn_bundle(SpriteBundle {
                    material: materials.add(Color::rgb_u8(52, 235, 140).into()),
                    sprite: Sprite::new(Vec2::new(
                        0.025 * window_size_diag.0,
                        0.025 * window_size_diag.0,
                    )),
                    transform: Transform {
                        translation: Vec3::new(
                            rng.gen_range(-window_size.width / 2.0..window_size.width / 2.0),
                            rng.gen_range(
                                window_size.height / 2.0 + camera_y
                                    ..window_size.height / 2.0 + camera_y + 32.0,
                            ),
                            0.,
                        ),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Food);
        }
    }
}

// moves the camera upwards all the time
fn camera_movement(
    mut query_set: QuerySet<(
        Query<(&mut Transform, With<OrthographicProjection>)>,
        Query<(&Score, With<Player>)>,
    )>,
    window_size_diag: Res<WindowSizeDiagonalWeighted>,
    time: Res<Time>,
) {
    let mut score = 0;
    if let Ok((player_score, _)) = query_set.q1_mut().single_mut() {
        score = player_score.0;
    }
    if let Ok((mut transform, _)) = query_set.q0_mut().single_mut() {
        transform.translation.y +=
            time.delta_seconds() * CAMERA_SPEED * (1.0 + 0.025 * score as f32) * window_size_diag.0
                / 720.0;
    }
}

// despawn all entities that are out of range for better performance
fn handle_entities_out_of_range(
    window_size: Res<WindowSize>,
    mut commands: Commands,
    mut query_set: QuerySet<(
        Query<(Entity, &Transform, &Sprite), Without<Food>>,
        Query<(&Transform, With<OrthographicProjection>)>,
        Query<(Entity, &Transform, &Sprite), With<Food>>,
    )>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    window_size_diag: Res<WindowSizeDiagonalWeighted>,
) {
    let mut camera_y = 0.0;
    if let Ok((camera_tf, _)) = query_set.q1_mut().single_mut() {
        camera_y = camera_tf.translation.y;
    }
    for (entity, transform, sprite) in query_set.q0_mut().iter_mut() {
        if transform.translation.y + sprite.size.y / 2.0 < camera_y - window_size.height / 2.0 {
            commands.entity(entity).despawn();
        }
    }
    let mut num_food = 0;
    for (entity, transform, sprite) in query_set.q2_mut().iter_mut() {
        num_food += 1;
        if transform.translation.y + sprite.size.y / 2.0 < camera_y - window_size.height / 2.0 {
            commands.entity(entity).despawn();
        }
    }
    if num_food == 0 {
        // spawn new food
        let mut rng = rand::thread_rng();
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.add(Color::rgb_u8(52, 235, 140).into()),
                sprite: Sprite::new(Vec2::new(
                    0.025 * window_size_diag.0,
                    0.025 * window_size_diag.0,
                )),
                transform: Transform {
                    translation: Vec3::new(
                        rng.gen_range(-window_size.width / 2.0..window_size.width / 2.0),
                        rng.gen_range(
                            window_size.height / 2.0 + camera_y
                                ..window_size.height / 2.0 + camera_y + 32.0,
                        ),
                        0.,
                    ),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Food);
    }
}

fn player_wall_collision(
    mut query_set: QuerySet<(
        Query<(&Transform, &Sprite, With<Wall>)>,
        Query<(&Transform, &Sprite, With<Player>)>,
    )>,
    mut player_lost_event: EventWriter<PlayerLostEvent>,
) {
    let mut player_pos = Vec3::ZERO;
    let mut player_size = Vec2::ZERO;
    if let Ok((tf, sprite, _)) = query_set.q1_mut().single_mut() {
        player_pos = tf.translation;
        player_size = sprite.size;
    }
    for (wall_tf, wall_sprite, __) in query_set.q0_mut().iter_mut() {
        if collide(
            player_pos,
            player_size,
            wall_tf.translation,
            wall_sprite.size,
        )
        .is_some()
        {
            // println!("Lost!");
            player_lost_event.send(PlayerLostEvent);
        }
    }
}

fn player_gate_collision(
    mut query_set: QuerySet<(
        Query<(&Transform, &Sprite, With<Player>)>,
        Query<(Entity, &Transform, &Sprite, With<Gate>)>,
    )>,
    mut gate_passed_event: EventWriter<GatePassedEvent>,
) {
    let mut player_pos = Vec3::ZERO;
    let mut player_size = Vec2::ZERO;
    if let Ok((tf, sprite, _)) = query_set.q0_mut().single_mut() {
        player_pos = tf.translation;
        player_size = sprite.size;
    }

    for (entity, gate_tf, gate_sprite, _) in query_set.q1_mut().iter_mut() {
        if collide(
            player_pos,
            player_size,
            gate_tf.translation,
            gate_sprite.size,
        )
        .is_some()
        {
            gate_passed_event.send(GatePassedEvent(entity));
        }
    }
}

fn gate_passed(
    mut commands: Commands,
    mut gate_passed_event: EventReader<GatePassedEvent>,
    mut player_query: Query<(&mut Score, With<Player>)>,
) {
    for ev in gate_passed_event.iter() {
        if let Ok((mut player_score, _)) = player_query.single_mut() {
            player_score.0 += 1;
            // println!("Score: {}", player_score.0);
            commands.entity(ev.0).despawn();
        }
    }
}

// checks if the player leaves the view
fn player_check_leave_view(
    mut query_set: QuerySet<(
        Query<&Transform, With<OrthographicProjection>>,
        Query<(&mut Transform, &mut Velocity, &Sprite), With<Player>>,
    )>,
    window_size: Res<WindowSize>,
    mut player_lost_event: EventWriter<PlayerLostEvent>,
) {
    let mut camera_y = 0.0;
    if let Ok(camera_tf) = query_set.q0_mut().single_mut() {
        camera_y = camera_tf.translation.y;
    }
    if let Ok((mut tf, mut velocity, sprite)) = query_set.q1_mut().single_mut() {
        if tf.translation.x - sprite.size.x/2.0 < -window_size.width/2.0 {
            velocity.0.x = 0.0;
            velocity.0.y = 0.0;
            tf.translation.x = -window_size.width/2.0 + sprite.size.x/2.0;
        }
        else if tf.translation.x + sprite.size.x/2.0 > window_size.width/2.0 {
            velocity.0.x = 0.0;
            velocity.0.y = 0.0;
            tf.translation.x = window_size.width/2.0 - sprite.size.x/2.0;
        }
        if tf.translation.y - sprite.size.y/2.0 < camera_y - window_size.height / 2.0 {
            // println!("Player lost!");
            player_lost_event.send(PlayerLostEvent.into());
        }
        else if tf.translation.y + sprite.size.y/2.0 > camera_y + window_size.height / 2.0 {
            velocity.0.x = 0.0;
            velocity.0.y = 0.0;
            tf.translation.y = camera_y + window_size.height/2.0 - sprite.size.x/2.0;
        }
    }
}

fn spawn_walls(
    window_size: Res<WindowSize>,
    time: Res<Time>,
    mut last_wall_spawned_at: ResMut<LastWallSpawnedAt>,
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query_set: QuerySet<(
        Query<(&mut Transform, With<OrthographicProjection>)>,
        Query<(&mut Score, With<Player>)>,
    )>,
    window_size_diag: Res<WindowSizeDiagonalWeighted>,
) {
    let time_now = time.seconds_since_startup();
    let mut score = 0;
    if let Ok((player_score, _)) = query_set.q1_mut().single_mut() {
        score = player_score.0;
    }
    if time_now - last_wall_spawned_at.0 > (2.0 - 0.1 * (score as f64)).max(1.0) {
        last_wall_spawned_at.0 = time_now;
        let mut rng = rand::thread_rng();
        let gap_middle = rng.gen_range(-0.3f32..0.3f32);
        let gap_width = 0.2f32;
        let mut camera_y = 0.0;
        if let Ok((camera_tf, _)) = query_set.q0_mut().single_mut() {
            camera_y = camera_tf.translation.y;
        }
        let wall_y = rng.gen_range(
            window_size.height / 2.0 + camera_y..window_size.height / 2.0 + camera_y + 32.0,
        );
        let gap_left = gap_middle - gap_width / 2.0;
        let gap_right = gap_middle + gap_width / 2.0;
        let wall_x_left = 0.5 * window_size.width * (-0.5 + gap_left);
        let wall_x_right = 0.5 * window_size.width * (0.5 + gap_right);
        let wall_width_left = window_size.width * (0.5 + gap_left);
        let wall_width_right = window_size.width * (0.5 - gap_right);
        let wall_height: f32 = window_size_diag.0 * 0.025;
        // println!("gl{} gr{} xl{} xr{} wl{} wr{} wsum{}", gap_left, gap_right, wall_x_left/window_size.width, wall_x_right/window_size.width, wall_width_left, wall_width_right, wall_width_left+wall_width_right);
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.add(Color::rgb_u8(0xE5, 0xE7, 0xEB).into()),
                sprite: Sprite::new(Vec2::new(wall_width_left, wall_height)),
                transform: Transform {
                    translation: Vec3::new(wall_x_left, wall_y, 0.1),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Wall);
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.add(Color::rgb_u8(0xE5, 0xE7, 0xEB).into()),
                sprite: Sprite::new(Vec2::new(wall_width_right, wall_height)),
                transform: Transform {
                    translation: Vec3::new(wall_x_right, wall_y, 0.1),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Wall);
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.add(Color::rgba_u8(0x10, 0xB9, 0x81, 64).into()),
                sprite: Sprite::new(Vec2::new(
                    window_size.width - wall_width_left - wall_width_right,
                    wall_height,
                )),
                transform: Transform {
                    translation: Vec3::new(
                        (wall_x_left + wall_width_left / 2.0 + wall_x_right
                            - wall_width_right / 2.0)
                            / 2.0,
                        wall_y,
                        0.1,
                    ),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Gate);
    }
}

fn player_lost(
    mut player_lost_event: EventReader<PlayerLostEvent>,
    mut app_state: ResMut<State<AppState>>,
    audio: Res<Audio>,
    death_sound: Res<DeathSound>,
) {
    for _ev in player_lost_event.iter() {
        app_state.set(AppState::Dead.into());
        audio.play(death_sound.0.clone());
        return;
    }
}

fn exit_playing(mut commands: Commands, mut query: Query<(Entity, With<OrthographicProjection>)>) {
    // println!("Exit Ingame");
    for (entity, _) in query.iter_mut() {
        commands.entity(entity).despawn_recursive();
    }
}

fn exit_deathscreen(mut commands: Commands, mut query: Query<Entity>) {
    for entity in query.iter_mut() {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_deathscreen_ui(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut query: Query<(&Score, With<Player>)>,
    window_size_diag: Res<WindowSizeDiagonalWeighted>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    button_materials: Res<ButtonMaterials>,
) {
    commands.spawn_bundle(UiCameraBundle::default());

    let mut score = 0;
    if let Ok((s, _)) = query.single_mut() {
        score = s.0;
    }

    // UI
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceAround,
                align_content: AlignContent::Center,
                ..Default::default()
            },
            material: materials.add(Color::rgb_u8(0x37, 0x41, 0x51).into()),
            ..Default::default()
        })
        .with_children(|parent| {
            // Score <x>
            parent.spawn_bundle(TextBundle {
                text: Text {
                    alignment: TextAlignment {
                        horizontal: HorizontalAlign::Center,
                        vertical: VerticalAlign::Center,
                    },
                    sections: vec![TextSection {
                        value: format!("Score {}", score),
                        style: TextStyle {
                            font_size: window_size_diag.0 * 0.07,
                            font: asset_server.load("fonts/BaiJamjuree-Medium.ttf"),
                            color: Color::rgb_u8(0xD1, 0xD5, 0xDB),
                        },
                    }],
                },
                ..Default::default()
            });
            // Respawn button
            parent
                .spawn_bundle(ButtonBundle {
                    style: Style {
                        // center button
                        padding: Rect::all(Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    material: button_materials.normal.clone(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        text: Text::with_section(
                            "Respawn",
                            TextStyle {
                                font: asset_server.load("fonts/BaiJamjuree-Medium.ttf"),
                                font_size: window_size_diag.0 * 0.07,
                                color: Color::rgb_u8(0xD1, 0xD5, 0xDB),
                            },
                            TextAlignment::default(),
                        ),
                        ..Default::default()
                    });
                });
            // title
            parent.spawn_bundle(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        style: TextStyle {
                            font_size: window_size_diag.0 * 0.12,
                            font: asset_server.load("fonts/BaiJamjuree-Bold.ttf"),
                            color: Color::rgb(1.0, 1.0, 1.0),
                        },
                        value: "Crashed!".into(),
                    }],
                    alignment: TextAlignment {
                        horizontal: HorizontalAlign::Center,
                        vertical: VerticalAlign::Center,
                    },
                    ..Default::default()
                },
                ..Default::default()
            });
        });
}

fn despawn_deathscreen_ui(mut commands: Commands, mut query: Query<Entity>) {
    for entity in query.iter_mut() {
        commands.entity(entity).despawn_recursive();
    }
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut tf, vel) in query.iter_mut() {
        tf.translation += vel.0 * time.delta_seconds();
    }
}

fn friction(mut query: Query<&mut Velocity>, time: Res<Time>) {
    for mut vel in query.iter_mut() {
        let vel_length = vel.0.length();
        if vel_length < VEL_CLIPPING {
            continue;
        }
        // vel.0 *= FRICTION;
        vel.0 *= VEL_CLIPPING.powf(time.delta_seconds());
        if vel.0.length() < VEL_CLIPPING {
            vel.0 = Vec3::ZERO;
        }
    }
}

fn gesture_on_player(
    mouse_buttons: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut drag_gesture: ResMut<DragGesture>,
    mut query_set: QuerySet<(
        Query<(&Transform, &Sprite, &mut Velocity, With<Player>)>,
        Query<(&Transform, With<OrthographicProjection>)>,
        Query<(Entity, &mut Transform, &mut Sprite), With<GestureLine>>,
    )>,
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let window = windows.get_primary().unwrap();
    let mut camera_y = 0.0f32;
    if let Ok((tf, _)) = query_set.q1_mut().single_mut() {
        camera_y = tf.translation.y;
    }
    let mut is_left_pressed = false;
    let mut player_pos_real = Vec3::ZERO;
    if let Ok((tf, _sprite, mut vel, _)) = query_set.q0_mut().single_mut() {
        let mut player_pos = tf.translation.clone();
        player_pos_real = tf.translation.clone();
        player_pos.y -= camera_y;
        is_left_pressed = mouse_buttons.pressed(MouseButton::Left);
        if is_left_pressed {
            if !drag_gesture.is_dragging {
                if let Some(_pos) = window.cursor_position() {
                    drag_gesture.start_pos = _pos;
                    drag_gesture.is_dragging = true;
                }

                // spawn line
                commands
                    .spawn_bundle(SpriteBundle {
                        sprite: Sprite::new(Vec2::new(3.0, 0.0)),
                        material: materials
                            .add(Color::rgba_u8(0xD1, 0xD5, 0xDB, 0x80).into())
                            .clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 1.0),
                        ..Default::default()
                    })
                    .insert(GestureLine);
            }
        }
        if !mouse_buttons.pressed(MouseButton::Left) {
            if drag_gesture.is_dragging {
                drag_gesture.is_dragging = false;
                if let Some(_pos) = window.cursor_position() {
                    let drag_vector = _pos - drag_gesture.start_pos;
                    vel.0.x -= drag_vector.x * PLAYER_SPEED;
                    vel.0.y -= drag_vector.y * PLAYER_SPEED;
                }

                // despawn line
                for (entity, _, __) in query_set.q2_mut().iter_mut() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    // draw line
    if is_left_pressed {
        for (_, mut tf, mut sprite) in query_set.q2_mut().iter_mut() {
            if let Some(mouse_pos) = window.cursor_position() {
                let diff = mouse_pos - drag_gesture.start_pos;
                let length = diff.length();
                let center = Vec2::new(player_pos_real.x, player_pos_real.y) + 0.5 * diff;
                let rotation = diff.angle_between(Vec2::Y);

                tf.translation.x = center.x;
                tf.translation.y = center.y;
                tf.rotation = Quat::from_rotation_z(-rotation);
                sprite.size.y = length;
            }
        }
    }
}

fn button_system(
    button_materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&Interaction, &mut Handle<ColorMaterial>, &Children),
        (Changed<Interaction>, With<Button>),
    >,
    mut app_state: ResMut<State<AppState>>,
) {
    for (interaction, mut material, _children) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                *material = button_materials.pressed.clone();
                if app_state.current() == &AppState::Paused {
                    app_state.pop();
                }
                else if app_state.current() == &AppState::Dead { 
                    app_state.set(AppState::Playing.into());
                }
            }
            Interaction::Hovered => {
                *material = button_materials.hovered.clone();
            }
            Interaction::None => {
                *material = button_materials.normal.clone();
            }
        }
    }
}

fn score_change(
    audio: Res<Audio>,
    score_sound: Res<ScoreSound>,
    mut query: Query<&Score, (With<Player>, Changed<Score>)>,
) {
    if let Ok(score) = query.single_mut() {
        if score.0 > 0 {
            // audio.play(asset_server.load("sounds/pickupCoin.wav").into());
            audio.play(score_sound.0.clone())
        }
    }
}

fn handle_resize(
    mut resize_event: EventReader<WindowResized>,
    mut window_size_diag: ResMut<WindowSizeDiagonalWeighted>,
    mut window_size: ResMut<WindowSize>,
) {
    for ev in resize_event.iter() {
        // println!("Resize: w={}, h={}", ev.width, ev.height);
        window_size.width = ev.width;
        window_size.height = ev.height;
        window_size_diag.0 = ((ev.width * ev.width + ev.height * ev.height) / 2.0).sqrt();
    }
}

fn window_focus(
    mut window_focused: EventReader<WindowFocused>,
    mut app_state: ResMut<State<AppState>>
) {
    for ev in window_focused.iter() {
        if !ev.focused && app_state.current().clone() == AppState::Playing {
            app_state.push(AppState::Paused.into());
        }
    }
}

fn esc_pause_check(
    input_buttons: Res<Input<KeyCode>>,
    mut app_state: ResMut<State<AppState>>
) {
    if input_buttons.just_pressed(KeyCode::Escape) {
        app_state.push(AppState::Paused.into());
    }
}


fn spawn_pause_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut query: Query<&Transform, With<OrthographicProjection>>,
    window_size_diag: Res<WindowSizeDiagonalWeighted>
) {
    // println!("Spawn pause ui");

    let mut camera_y: f32 = 0.0;
    if let Ok(tf) = query.single_mut() {
        camera_y = tf.translation.y;
    }
    commands.spawn_bundle(Text2dBundle {
        text: Text {
            alignment: TextAlignment {
                horizontal: HorizontalAlign::Center,
                vertical: VerticalAlign::Center
            },
            sections: vec![
                TextSection {
                    value: "Continue".to_string(),
                    style: TextStyle {
                        color: Color::WHITE,
                        font: asset_server.load("fonts/BaiJamjuree-Medium.ttf"),
                        font_size: window_size_diag.0 * 0.07
                    }
                }
            ]
        },
        transform: Transform::from_xyz(0.0, camera_y, 99.0),
        ..Default::default()
    })
        .insert(PauseMenuText);
}

fn unpause_check(
    mouse_buttons: Res<Input<MouseButton>>,
    mut app_state: ResMut<State<AppState>>
) {
    if mouse_buttons.just_pressed(MouseButton::Left) {
        app_state.pop();
    }
}

fn despawn_pause_ui(
    mut commands: Commands,
    query: Query<Entity, (With<Text>, With<PauseMenuText>)>,
) {
    // println!("Despawn pause ui");
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn main() {
    App::build()
        // .insert_resource(Msaa { samples: 2 })
        .insert_resource(ClearColor(Color::rgb_u8(52, 103, 235)))
        .insert_resource(WindowDescriptor {
            title: "Endless game".to_string(),
            width: 720.0,
            height: 760.0,
            // mode: WindowMode::Fullscreen {use_size: false},
            resizable: true,
            vsync: true,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        // .add_plugin(AudioPlugin)
        .add_event::<FoodEatenEvent>()
        .add_event::<GatePassedEvent>()
        .add_event::<PlayerLostEvent>()
        .add_state(AppState::Playing)
        .add_system_set(
            SystemSet::on_enter(AppState::Playing).with_system(setup.system()),
        )
        .add_system_set(
            SystemSet::on_update(AppState::Playing)
                .with_system(apply_velocity.system())
                .with_system(friction.system())
                .with_system(camera_movement.system().label("camera_movement"))
                .with_system(player_check_food.system().label("check_food"))
                .with_system(food_eaten.system().after("check_food"))
                .with_system(player_check_leave_view.system())
                .with_system(handle_entities_out_of_range.system())
                .with_system(spawn_walls.system())
                .with_system(player_wall_collision.system())
                .with_system(player_gate_collision.system().label("gate_collision"))
                .with_system(gate_passed.system().after("gate_collision"))
                .with_system(gesture_on_player.system())
                .with_system(update_scoreboard.system().after("camera_movement"))
                .with_system(score_change.system())
                .with_system(handle_resize.system())
                .with_system(window_focus.system())
                .with_system(esc_pause_check.system())
                .with_system(player_lost.system()),
        )
        .add_system_set(SystemSet::on_exit(AppState::Playing).with_system(exit_playing.system()))
        .add_system_set(
            SystemSet::on_enter(AppState::Dead)
                .with_system(spawn_deathscreen_ui.system()),
        )
        .add_system_set(
            SystemSet::on_update(AppState::Dead)
                .with_system(button_system.system()),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::Dead)
                .with_system(exit_deathscreen.system())
                .with_system(despawn_deathscreen_ui.system()),
        )
        .add_system_set(
            SystemSet::on_enter(AppState::Paused)
                .with_system(spawn_pause_ui.system())
        )
        .add_system_set(
            SystemSet::on_update(AppState::Paused)
                .with_system(unpause_check.system()),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::Paused)
                .with_system(despawn_pause_ui.system()),
        )
        .run();
}
