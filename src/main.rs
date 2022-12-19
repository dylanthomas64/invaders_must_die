use std::f32::consts::PI;

use bevy::{math::Vec3Swizzles, prelude::*, sprite::collide_aabb::collide, utils::HashSet};
use components::{
    Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, FromEnemy, FromPlayer, Laser, Movable,
    Player, SpriteSize, Velocity, Orientation,
};
use enemy::EnemyPlugin;
use player::PlayerPlugin;

mod components;
mod enemy;
mod player;

// Asset Constants
const PLAYER_SPRITE: &str = "player_b_01.png";
const PLAYER_SIZE: (f32, f32) = (98., 75.);
const SPRITE_SCALE: f32 = 0.5;
const PLAYER_LASER_SPRITE: &str = "laser_a_01.png";
const PLAYER_LASER_SIZE: (f32, f32) = (9., 54.);
const ENEMY_SPRITE: &str = "enemy_a_01.png";
const ENEMY_SIZE: (f32, f32) = (93., 84.);
const ENEMY_LASER_SPRITE: &str = "laser_b_01.png";
const ENEMY_LASER_SIZE: (f32, f32) = (17., 55.);
const EXPLOSION: &str = "explo_a_sheet.png";
const EXPLOSION_LENGTH: usize = 16;

// game constants

const TIME_STEP: f32 = 1. / 60.;
const BASE_SPEED: f32 = 500.;

const PLAYER_RESPAWN_DELAY: f64 = 2.;
const ENEMY_MAX: u32 = 0;

// Resources

#[derive(Resource)]
pub struct WinSize {
    pub w: f32,
    pub h: f32,
}

#[derive(Resource)]
struct GameTextures {
    player: Handle<Image>,
    player_laser: Handle<Image>,
    enemy: Handle<Image>,
    explosion: Handle<TextureAtlas>,
    enemy_laser: Handle<Image>,
}

#[derive(Resource)]
struct EnemyCount(u32);

#[derive(Resource)]
struct PlayerState {
    on: bool,       // alive
    last_shot: f64, // -1 if not shot
    score: u32,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            on: false,
            last_shot: -1.,
            score: 0,
        }
    }
}

impl PlayerState {
    pub fn shot(&mut self, time: f64) {
        self.on = false;
        self.last_shot = time;
    }

    pub fn spawned(&mut self) {
        self.on = true;
        self.last_shot = -1.;
    }
}

#[derive(Component)]
pub struct PlayerScore;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "Invaders must die".to_string(),
                width: 598.0,
                height: 676.0,
                ..Default::default()
            },
            ..Default::default()
        }))
        .add_plugin(PlayerPlugin)
        .add_plugin(EnemyPlugin)
        .add_startup_system(setup_system)
        .add_system(gamepad_connections)
        .add_system(gamepad_input)
        .add_system(moveable_system.after(gamepad_input))
        .add_system(player_laser_hit_enemy_system)
        .add_system(explosion_to_spawn_system)
        .add_system(explosion_animation_system)
        .add_system(enemy_laser_hit_player_system)
        .add_system(enemy_player_collision_system)
        .add_system(player_score_update_system)
        .run();
}

fn setup_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut windows: ResMut<Windows>,
) {
    // 2d camera
    commands.spawn(Camera2dBundle::default());

    // Text with multiple sections
    commands.spawn((
        // Create a TextBundle that has a Text with a list of sections.
        TextBundle::from_sections([
            TextSection::new(
                "Score: ",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Black.ttf"),
                    font_size: 60.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::from_style(TextStyle {
                font: asset_server.load("fonts/FiraSans-Black.ttf"),
                font_size: 60.0,
                color: Color::GOLD,
            }),
        ]),
        PlayerScore,
    ));

    //capture window size
    let window = windows.get_primary_mut().unwrap();
    let (win_w, win_h) = (window.width(), window.height());

    // position window
    // window.set_position(IVec2::new(800, 0));

    // add winsize resource
    let win_size = WinSize { w: win_w, h: win_h };
    commands.insert_resource(win_size);

    // create explosion texture atlas
    let texture_handle = asset_server.load(EXPLOSION);
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(64., 64.), 4, 4, None, None);
    let explosion = texture_atlases.add(texture_atlas);

    // add game textures resource
    let game_textures = GameTextures {
        player: asset_server.load(PLAYER_SPRITE),
        player_laser: asset_server.load(PLAYER_LASER_SPRITE),
        enemy: asset_server.load(ENEMY_SPRITE),
        explosion,
        enemy_laser: asset_server.load(ENEMY_LASER_SPRITE),
    };
    commands.insert_resource(game_textures);
    commands.insert_resource(EnemyCount(0));
}

fn moveable_system(
    mut commands: Commands,
    win_size: Res<WinSize>,
    mut query: Query<(Entity, &Velocity, &Orientation, &mut Transform, &Movable)>,
) {
    for (entity, velocity, orientation, mut transform, movable) in query.iter_mut() {
        let translation = &mut transform.translation;
        translation.x += velocity.x * TIME_STEP * BASE_SPEED;
        translation.y += velocity.y * TIME_STEP * BASE_SPEED;

        if movable.auto_despawn {
            //despawn when off screen
            const MARGIN: f32 = 200.;
            if translation.y > win_size.h / 2. + MARGIN
                || translation.y < -win_size.h / 2. - MARGIN
                || translation.x > win_size.w / 2. + MARGIN
                || translation.x < -win_size.w / 2. - MARGIN
            {
                //println!("--> despawn {entity:?} @ {translation:?}");
                commands.entity(entity).despawn();
            }
        }

        transform.rotation = Quat::from_rotation_z(orientation.theta);

    }
}

fn player_laser_hit_enemy_system(
    mut commands: Commands,
    mut enemy_count: ResMut<EnemyCount>,
    mut player_state: ResMut<PlayerState>,
    laser_query: Query<(Entity, &Transform, &SpriteSize), (With<Laser>, With<FromPlayer>)>,
    enemy_query: Query<(Entity, &Transform, &SpriteSize), With<Enemy>>,
) {
    let mut despawned_enemies: HashSet<Entity> = HashSet::new();

    // iterate through the lasers
    for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
        if despawned_enemies.contains(&laser_entity) {
            continue;
        }
        let laser_scale = laser_tf.scale.xy();

        // iterate through enemies
        for (enemy_entity, enemy_tf, enemy_size) in enemy_query.iter() {
            if despawned_enemies.contains(&enemy_entity)
                || despawned_enemies.contains(&laser_entity)
            {
                continue;
            }
            let enemy_scale = enemy_tf.scale.xy();

            // determine if collision
            let collision = collide(
                laser_tf.translation,
                laser_size.0 * laser_scale,
                enemy_tf.translation,
                enemy_size.0 * enemy_scale,
            );

            // perform collision logic

            if collision.is_some() {
                commands.entity(enemy_entity).despawn();
                despawned_enemies.insert(enemy_entity);
                enemy_count.0 -= 1;

                commands.entity(laser_entity).despawn();
                despawned_enemies.insert(laser_entity);

                // add to score
                player_state.score += 1;

                // spawn explosionToSpawn
                commands.spawn(ExplosionToSpawn(enemy_tf.translation));
            }
        }
    }
}

fn enemy_laser_hit_player_system(
    mut commands: Commands,
    mut player_state: ResMut<PlayerState>,
    time: Res<Time>,
    laser_query: Query<(Entity, &Transform, &SpriteSize), (With<Laser>, With<FromEnemy>)>,
    player_query: Query<(Entity, &Transform, &SpriteSize), With<Player>>,
) {
    if let Ok((player_entity, player_tf, player_size)) = player_query.get_single() {
        let player_scale = player_tf.scale.xy();

        for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
            let laser_scale = laser_tf.scale.xy();

            //determine if collision
            let collision = collide(
                player_tf.translation,
                player_size.0 * player_scale,
                laser_tf.translation,
                laser_size.0 * laser_scale,
            );

            //perform the collision
            if collision.is_some() {
                // despawn player and laser
                commands.entity(player_entity).despawn();
                player_state.shot(time.elapsed_seconds_f64());

                commands.entity(laser_entity).despawn();

                // spawn explosion
                commands.spawn(ExplosionToSpawn(player_tf.translation));

                break;
            }
        }
    }
}

fn explosion_to_spawn_system(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    query: Query<(Entity, &ExplosionToSpawn)>,
) {
    for (explosion_spawn_entity, explosion_to_spawn) in query.iter() {
        //spawn explosion sprite
        commands
            .spawn(SpriteSheetBundle {
                texture_atlas: game_textures.explosion.clone(),
                transform: Transform {
                    translation: explosion_to_spawn.0,
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Explosion)
            .insert(ExplosionTimer::default());

        // despawn the explosionToSpawn
        commands.entity(explosion_spawn_entity).despawn();
    }
}

fn explosion_animation_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ExplosionTimer, &mut TextureAtlasSprite), With<Explosion>>,
) {
    for (entity, mut timer, mut sprite) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            sprite.index += 1; // move to next sprite cell
            if sprite.index >= EXPLOSION_LENGTH {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn enemy_player_collision_system(
    mut commands: Commands,
    mut player_state: ResMut<PlayerState>,
    mut enemy_count: ResMut<EnemyCount>,
    time: Res<Time>,
    enemy_query: Query<(Entity, &Transform, &SpriteSize), With<Enemy>>,
    player_query: Query<(Entity, &Transform, &SpriteSize), With<Player>>,
) {
    if let Ok((player_entity, player_tf, player_size)) = player_query.get_single() {
        let player_scale = player_tf.scale.xy();

        for (enemy_entity, enemy_tf, enemy_size) in enemy_query.iter() {
            let laser_scale = enemy_tf.scale.xy();

            //determine if collision
            let collision = collide(
                player_tf.translation,
                player_size.0 * player_scale,
                enemy_tf.translation,
                enemy_size.0 * laser_scale,
            );

            //perform the collision
            if collision.is_some() {
                // despawn player and laser
                commands.entity(player_entity).despawn();
                player_state.shot(time.elapsed_seconds_f64());

                commands.entity(enemy_entity).despawn();

                // spawn explosion
                commands.spawn(ExplosionToSpawn(player_tf.translation));
                commands.spawn(ExplosionToSpawn(enemy_tf.translation));

                enemy_count.0 -= 1;

                break;
            }
        }
    }
}

fn player_score_update_system(
    player_state: Res<PlayerState>,
    mut query: Query<&mut Text, With<PlayerScore>>,
) {
    for mut text in &mut query {
        text.sections[1].value = player_state.score.to_string();
    }
}

/// Simple resource to store the ID of the connected gamepad.
/// We need to know which gamepad to use for player input.
#[derive(Resource)]
struct MyGamepad(Gamepad);

fn gamepad_connections(
    mut commands: Commands,
    my_gamepad: Option<Res<MyGamepad>>,
    mut gamepad_evr: EventReader<GamepadEvent>,
) {
    for ev in gamepad_evr.iter() {
        // the ID of the gamepad
        let id = ev.gamepad;
        match &ev.event_type {
            GamepadEventType::Connected(info) => {
                println!("New gamepad connected with ID: {:?}, name: {}", id, info.name);

                // if we don't have any gamepad yet, use this one
                if my_gamepad.is_none() {
                    commands.insert_resource(MyGamepad(id));
                }
            }
            GamepadEventType::Disconnected => {
                println!("Lost gamepad connection with ID: {:?}", id);

                // if it's the one we previously associated with the player,
                // disassociate it:
                if let Some(MyGamepad(old_id)) = my_gamepad.as_deref() {
                    if *old_id == id {
                        commands.remove_resource::<MyGamepad>();
                    }
                }
            }
            // other events are irrelevant
            _ => {}
        }
    }
}

fn gamepad_input(
    axes: Res<Axis<GamepadAxis>>,
    buttons: Res<Input<GamepadButton>>,
    my_gamepad: Option<Res<MyGamepad>>,
    mut query: Query<(&mut Velocity, &mut Orientation), With<Player>>,
) {
    let gamepad = if let Some(gp) = my_gamepad {
        // a gamepad is connected, we have the id
        gp.0
    } else {
        // no gamepad is connected
        return;
    };

    // The joysticks are represented using a separate axis for X and Y
    let axis_lx = GamepadAxis {
        gamepad, axis_type: GamepadAxisType::LeftStickX
    };
    let axis_ly = GamepadAxis {
        gamepad, axis_type: GamepadAxisType::LeftStickY
    };
    
    let axis_rx = GamepadAxis {
        gamepad, axis_type: GamepadAxisType::RightStickX
    };
    let axis_ry = GamepadAxis {
        gamepad, axis_type: GamepadAxisType::RightStickY
    };

    for (mut velocity, mut orientation) in query.iter_mut() {
        if let (Some(x), Some(y)) = (axes.get(axis_lx), axes.get(axis_ly)) {
            velocity.x = x;
            velocity.y = y;
        };
        
        if let (Some(x), Some(y)) = (axes.get(axis_rx), axes.get(axis_ry)) {
            let acute_angle = if x == 0. {
                // if x is 0 then return to previous state and avoid division by zero
                return;
            } else {
                y.abs()/x.abs().atan()
            };
            let acute_angle = (y.abs()/x.abs()).atan();
            let theta = if y.is_sign_positive() {
                // quadrant I
                if x.is_sign_positive() {
                    acute_angle
                } else { // quadrant II
                    PI - acute_angle
                }
            } else {
                // quadrant III
                if x.is_sign_negative() {
                    PI + acute_angle
                } else { // quadrant IV
                    (2. * PI) - acute_angle
                }
            };
            println!("{}", theta);
            // account for sprite starting with Pi/ 2 rotation
            orientation.theta = theta - ( PI / 2.)
        }  
    }


    /* 
    // In a real game, the buttons would be configurable, but here we hardcode them
    let jump_button = GamepadButton {
        gamepad, button_type: GamepadButtonType::South
    };
    let heal_button = GamepadButton {
        gamepad, button_type: GamepadButtonType::East
    };

    if buttons.just_pressed(jump_button) {
        // button just pressed: make the player jump
    }

    if buttons.pressed(heal_button) {
        // button being held down: heal the player
    } */
}