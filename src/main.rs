use bevy::{math::Vec3Swizzles, prelude::*, sprite::collide_aabb::{collide, Collision}, utils::HashSet};
use components::{Enemy, FromPlayer, Laser, Movable, SpriteSize, Velocity, ExplosionToSpawn, ExplosionTimer, Explosion, FromEnemy, Player};
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
const ENEMY_LASER_SIZE: (f32, f32) =(17., 55.);
const EXPLOSION: &str = "explo_a_sheet.png";
const EXPLOSION_LENGTH: usize = 16;



// game constants

const TIME_STEP: f32 = 1. / 60.;
const BASE_SPEED: f32 = 500.;

const PLAYER_RESPAWN_DELAY: f64 =  2.;
const ENEMY_MAX: u32 = 2;

// Resources
pub struct WinSize {
    pub w: f32,
    pub h: f32,
}

struct GameTextures {
    player: Handle<Image>,
    player_laser: Handle<Image>,
    enemy: Handle<Image>,
    explosion: Handle<TextureAtlas>,
    enemy_laser: Handle<Image>,
}
struct EnemyCount(u32);

struct PlayerState {
    on: bool, // alive
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




fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .insert_resource(WindowDescriptor {
            title: "Invaders must die".to_string(),
            width: 598.0,
            height: 676.0,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(PlayerPlugin)
        .add_plugin(EnemyPlugin)
        .add_startup_system(setup_system)
        .add_system(moveable_system)
        .add_system(player_laser_hit_enemy_system)
        .add_system(explosion_to_spawn_system)
        .add_system(explosion_animation_system)
        .add_system(enemy_laser_hit_player_system)
        .add_system(enemy_player_collision_system)
        .run();
}

fn setup_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut windows: ResMut<Windows>,
) {
    // 2d camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());




    //capture window size
    let window = windows.get_primary_mut().unwrap();
    let (win_w, win_h) = (window.width(), window.height());

    // position window
    window.set_position(IVec2::new(800, 0));

    // add winsize resource
    let win_size = WinSize { w: win_w, h: win_h };
    commands.insert_resource(win_size);

    // create explosion texture atlas
    let texture_handle = asset_server.load(EXPLOSION);
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(64., 64.), 4, 4);
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
    mut query: Query<(Entity, &Velocity, &mut Transform, &Movable)>,
) {
    for (entity, velocity, mut transform, movable) in query.iter_mut() {
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
        let laser_scale = Vec2::from(laser_tf.scale.xy());

        // iterate through enemies
        for (enemy_entity, enemy_tf, enemy_size) in enemy_query.iter() {
            if despawned_enemies.contains(&enemy_entity) || despawned_enemies.contains(&laser_entity) {
                continue;
            }
            let enemy_scale = Vec2::from(enemy_tf.scale.xy());

            // determine if collision
            let collision = collide(
                laser_tf.translation,
                laser_size.0 * laser_scale,
                enemy_tf.translation,
                enemy_size.0 * laser_scale,
            );

            // perform collision logic

            if let Some(_) = collision {
                commands.entity(enemy_entity).despawn();
                despawned_enemies.insert(enemy_entity);
                enemy_count.0 -= 1;

                commands.entity(laser_entity).despawn();
                despawned_enemies.insert(laser_entity);

                // add to score
                player_state.score += 1;

                // spawn explosionToSpawn
                commands.spawn().insert(ExplosionToSpawn(enemy_tf.translation.clone()));
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
        let player_scale = Vec2::from(player_tf.scale.xy());

        for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
            let laser_scale = Vec2::from(laser_tf.scale.xy());

            //determine if collision
            let collision = collide(
                player_tf.translation,
                player_size.0 * player_scale,
                laser_tf.translation,
                laser_size.0 * laser_scale,

            );

            //perform the collision
            if let Some(_) = collision {
                // despawn player and laser
                commands.entity(player_entity).despawn();
                player_state.shot(time.seconds_since_startup());

                commands.entity(laser_entity).despawn();

                // spawn explosion
                commands.spawn().insert(ExplosionToSpawn(player_tf.translation.clone()));

                break;
            }
        }
    }
}

fn explosion_to_spawn_system(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    query: Query<(Entity, &ExplosionToSpawn)>
) {
    for (explosion_spawn_entity, explosion_to_spawn) in query.iter() {
        //spawn explosion sprite
        commands.spawn_bundle(SpriteSheetBundle {
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
    mut query: Query<(Entity, &mut ExplosionTimer, &mut TextureAtlasSprite), With<Explosion>>
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
        let player_scale = Vec2::from(player_tf.scale.xy());

        for (enemy_entity, enemy_tf, enemy_size) in enemy_query.iter() {
            let laser_scale = Vec2::from(enemy_tf.scale.xy());

            //determine if collision
            let collision = collide(
                player_tf.translation,
                player_size.0 * player_scale,
                enemy_tf.translation,
                enemy_size.0 * laser_scale,

            );

            //perform the collision
            if let Some(_) = collision {
                // despawn player and laser
                commands.entity(player_entity).despawn();
                player_state.shot(time.seconds_since_startup());

                commands.entity(enemy_entity).despawn();

                // spawn explosion
                commands.spawn().insert(ExplosionToSpawn(player_tf.translation.clone()));
                commands.spawn().insert(ExplosionToSpawn(enemy_tf.translation.clone()));

                enemy_count.0 -= 1;

                break;
            }
        }
    }
}