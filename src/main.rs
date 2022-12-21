use std::f32::consts::PI;

use bevy::{math::Vec3Swizzles, prelude::*, sprite::collide_aabb::collide, utils::HashSet};
use components::{
    Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, FromEnemy, FromPlayer, Laser, Movable,
    Orientation, Player, SpriteSize, Velocity,
};
use enemy::EnemyPlugin;
use player::PlayerPlugin;

use bevy_rapier2d::prelude::*;

use std::collections::HashMap;


//#[deny(warnings)]

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

const G: f32 = 0.00000000006674;
const PRIMARY_THRUST: f32 = 100_000.; // left trigger
const SECONDARY_THRUST: f32 = 10_000.; // thumbstick adjustments

const LASER_VELOCITY: f32 = 100.;

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
                width: 1080.0,
                height: 720.0,
                ..Default::default()
            },
            ..Default::default()
        }))
        .add_plugin(PlayerPlugin)
        .add_plugin(EnemyPlugin)
        .add_startup_system(setup_system)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(2.0))
        .insert_resource(RapierConfiguration {
            gravity: Vec2::new(0., 0.),
            ..Default::default()
        })
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_startup_system(setup_physics)
        //.add_system(print_ball_altitude)
        .add_system(apply_gravitational_forces)
        .add_system(gamepad_connections)
        .add_system(gamepad_input)
        //.add_system(moveable_system.after(gamepad_input))
        .add_system(despawn_system.after(apply_gravitational_forces))
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


fn setup_physics(mut commands: Commands) {

    // FLOOR
    /*
    commands.spawn(Collider::cuboid(500., 10.))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -300., 0.0)));
        */
    
    // LARGE DENSE BALL
    let mut spawn_dense_ball = |x: f32, y: f32, r, d| {
        commands.spawn(RigidBody::Dynamic)
        .insert(Collider::ball(r))
        .insert(TransformBundle::from(Transform::from_xyz(x, y, 0.)))
        .insert(Velocity {
            x: 0.,
            y: -3.0,
        })
        .insert(Sleeping::disabled())
        .insert(Ccd::enabled())
        .insert(Restitution::coefficient(0.8))
        .insert(ExternalForce {
            force: Vec2::new(0., -100.),
            torque: 0.0,
        })
        .insert(ExternalImpulse {
            impulse: Vec2::new(0.0, 0.0),
            torque_impulse: 0.,
        })
        .insert(ColliderMassProperties::Density(1.0))
        .insert(ReadMassProperties(MassProperties {
            ..Default::default()
        }));
    };
    
    spawn_dense_ball(-30., 0., 50., 1.0);
    spawn_dense_ball(150., 60., 75., 1.3);
    spawn_dense_ball(0., 30., 45., 1.5);



    // LIGHT ball
    let mut spawn_light_ball = |x, y| {
        commands.spawn(RigidBody::Dynamic)
            .insert(Collider::ball(5.))
            .insert(TransformBundle::from(Transform::from_xyz(x, y, 0.,)))
            .insert(ExternalForce {
                force: Vec2::new(0., -100.),
                torque: 0.
            })
            .insert(Restitution::coefficient(0.8))
            .insert(ColliderMassProperties::Density(1.0))
            .insert(ExternalImpulse {
                impulse: Vec2::new(0.0, 0.0),
                torque_impulse: 0.,
            })
            .insert(ReadMassProperties(MassProperties {
                ..Default::default()
            }));
    };
    for n in -8..8 {
        spawn_light_ball(100.*n as f32, 100.);
    }

    // cuboid
    let mut spawn_cuboid = |x, y| {
        commands.spawn(RigidBody::Dynamic)
            .insert(Collider::cuboid(10., 20.))
            .insert(TransformBundle::from(Transform::from_xyz(x, y, 0.,)))
            .insert(ExternalForce {
                force: Vec2::new(0., -100.),
                torque: 0.
            })
            .insert(Restitution::coefficient(0.8))
            .insert(ColliderMassProperties::Density(1.0))
            .insert(ExternalImpulse {
                impulse: Vec2::new(0.0, 0.0),
                torque_impulse: 0.,
            })
            .insert(ReadMassProperties(MassProperties {
                ..Default::default()
            }));
    };
    for n in -3..3 {
        spawn_cuboid(200.*n as f32, -100.);
    }

    let mut spawn_triangle = |x, y| {
        commands.spawn(RigidBody::Dynamic)
            .insert(Collider::triangle(Vec2::new(20., 20.), Vec2::new(20., 20.), Vec2::new(20., 20.)))
            .insert(TransformBundle::from(Transform::from_xyz(x, y, 0.,)))
            .insert(ExternalForce {
                force: Vec2::new(0., -100.),
                torque: 0.
            })
            .insert(Restitution::coefficient(0.8))
            .insert(ColliderMassProperties::Density(1.0))
            .insert(ExternalImpulse {
                impulse: Vec2::new(0.0, 0.0),
                torque_impulse: 0.,
            })
            .insert(ReadMassProperties(MassProperties {
                ..Default::default()
            }));
    };

    for n in -3..3 {
        //spawn_triangle(150.*n as f32, 0.);
    }

        
        
  
}





fn print_ball_altitude(positions: Query<(Entity, &Transform, &ReadMassProperties)>) {
    for (ent, tf, mass_prop) in positions.iter() {
        println!("{:?} {}Kg @ altitude: {}", ent, mass_prop.0.mass, tf.translation.y);
    }
}

// apply gravitational field in centrew
fn apply_central_gravity(
    mut ext_forces: Query<(&Transform, &mut ExternalForce)>
) {
    for (transform, mut ext_force) in ext_forces.iter_mut() {
        // apply force towards centre (0,0)
        let (x, y) = (transform.translation.x, transform.translation.y);        
        // calculate disance from centre
        let distance_squared = x.powf(2.) + y.powf(2.);
        println!("distance: {}", distance_squared);
        // calculate acute angle towards centre
        let acute_angle = if x == 0. {
            PI / 2.
        } else {
            (y.abs() / x.abs()).atan()
        };

        if distance_squared != 0. {
            // resolve x and y parts of force to create absolute force of 10N
        let force_x = (acute_angle.cos() * 10.);
        let force_y = (acute_angle.sin() * 10.);

         // calulate direction coefficient
         let x_s = if x.is_sign_positive() {
            -1.
        } else {
            1.
        };
        // calulate direction coefficient
        let y_s = if y.is_sign_positive() {
            -1.
        } else {
            1.
        };
        ext_force.force = Vec2::new(force_x*x_s, force_y*y_s);
        }
        

       


    }
}

const extra_gravity: f32 = 1_000_000_000_000.;

fn apply_gravity_for_two(
    mut query: Query<(&Transform, &ReadMassProperties)>,
    mut force_query: Query<&mut ExternalForce>,
) {
    let mut coordinates: Vec<(f32, f32)> = Vec::new();
    let mut masses: Vec<f32> = Vec::new();
    for (tf, mass) in query.iter() {
        //println!("{}kg @ ({},{})", mass.0.mass, tf.translation.x, tf.translation.y);
        coordinates.push((tf.translation.x, tf.translation.y));
        masses.push(mass.0.mass);
    }
    // dx, dy relative to first object
    let (dx, dy) = (coordinates[0].0 - coordinates[1].0, coordinates[0].1 - coordinates[1].1);
    
    let distance_squared = dx.powf(2.) + dy.powf(2.);
    // Newton's law of gravitation * extra strong gravity
    let force = (masses[0]*masses[1]*G) / distance_squared;
    //println!("{}m", distance_squared.sqrt());
    //println!("{force}N");
    let acute_angle = if dx == 0. {
        PI / 2.
    } else {
        (dy.abs() / dx.abs()).atan()
    };
    // resolve x and y forces
    let force_x = (acute_angle.cos() * force * extra_gravity);
    let force_y = (acute_angle.sin() * force * extra_gravity);

    // determine direction of force to apply
    let x_s = if dx.is_sign_positive() {
        -1.
    } else {
        1.
    };
    // calulate direction coefficient
    let y_s = if dy.is_sign_positive() {
        -1.
    } else {
        1.
    };
    
    let mut iter = force_query.iter_mut();
    
    // apply forces on first
    iter.next().unwrap().force = Vec2::new(force_x*x_s, force_x*y_s);
    // apply opposite forces on the second
    iter.next().unwrap().force = Vec2::new(-force_x*x_s, -force_x*y_s);

    println!("applying: ({}N,{}N)  and ({}N,{}N)", force_x*x_s, force_x*y_s, -force_x*x_s, -force_x*y_s)
}




fn apply_gravitational_forces(
    mut commands: Commands,
    query: Query<(Entity, &Transform, &ReadMassProperties)>,
) {
    let mut cumulative_force_hash_map = HashMap::new();
    let mut data: Vec<(Entity, (f32, f32), f32)> = Vec::new();
    for (ent, mut tf, mass_prop) in query.iter() {

        cumulative_force_hash_map.insert(ent, Vec2::new(0., 0.));
        //println!("tf: {:?}", tf);
        data.push((ent, (tf.translation.x, tf.translation.y), mass_prop.0.mass))
    }

    for (n_1, (ent_1, (x_1,y_1), mass_1)) in data.clone().iter().enumerate() {
        // ignore first n terms as have already been calcualted
        for (n_2, (ent_2, (x_2,y_2), mass_2)) in data.iter().enumerate().skip(n_1 + 1) {
            // return if comparing same entity
            //println!("({},{}), ({}, {})", x_1, y_1, x_2, y_2);
           
            //println!("{}vs{}", n_1, n_2);
            // change in (x, y) between two points
            let (dx, dy) = (x_1 - x_2, y_1 - y_2);
            //println!("dx:{}, dy:{}", dx, dy);
            // find distance squared using pythagoras
            let distance_squared = dx.powf(2.) + dy.powf(2.);
            // calculate force due to gravity
            let force = (mass_1*mass_2*G) / distance_squared;
            //println!("abs force: {}N", force);

            let acute_angle = if dx == 0. {
                PI / 2.
            } else {
                (dy.abs() / dx.abs()).atan()
            };
            // resolve x and y forces
            let force_x = acute_angle.cos() * force * extra_gravity;
            let force_y = acute_angle.sin() * force * extra_gravity;

            //println!("x: {}, y: {}", force_x, force_y);
        
            // determine direction of force to apply
            let x_s = if dx.is_sign_positive() {
                -1.
            } else {
                1.
            };
            // calulate direction coefficient
            let y_s = if dy.is_sign_positive() {
                -1.
            } else {
                1.
            };
            // force due to gravity on entity_1
            let force_1 = Vec2::new(force_x*x_s, force_x*y_s);
            // equal opposite force due to gravity on entity_2
            let force_2 = Vec2::new(-force_x*x_s, -force_x*y_s);
            
            // cumulatively adds force vectors to entities existing forces
            cumulative_force_hash_map.entry(*ent_1).and_modify(|force| *force += force_1);
            cumulative_force_hash_map.entry(*ent_2).and_modify(|force| *force += force_2);
            //println!("{}N applied on {:?} <-> {:?}", force, ent_1, ent_2);
        }

    }

    for (ent, force) in cumulative_force_hash_map.drain() {
        commands.get_entity(ent.clone()).unwrap().insert(ExternalForce{force: force, torque: 0.});
    }
}

fn despawn_system(
    mut commands: Commands,
    win_size: Res<WinSize>,
    mut query: Query<(Entity, &Transform), Without<Player>>
) {
     for (entity, tf) in query.iter_mut() {
        let translation = tf.translation;
            //despawn when off screen
            const MARGIN: f32 = 200.;
            if translation.y > win_size.h / 2. + MARGIN
                || translation.y < -win_size.h / 2. - MARGIN
                || translation.x > win_size.w / 2. + MARGIN
                || translation.x < -win_size.w / 2. - MARGIN
            {
                println!("--> despawn {entity:?} @ {translation:?}");
                commands.entity(entity).despawn();
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
                println!(
                    "New gamepad connected with ID: {:?}, name: {}",
                    id, info.name
                );

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
    mut query: Query<(&mut ExternalForce, &mut Transform, &mut Orientation), With<Player>>,
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
        gamepad,
        axis_type: GamepadAxisType::LeftStickX,
    };
    let axis_ly = GamepadAxis {
        gamepad,
        axis_type: GamepadAxisType::LeftStickY,
    };

    let axis_rx = GamepadAxis {
        gamepad,
        axis_type: GamepadAxisType::RightStickX,
    };
    let axis_ry = GamepadAxis {
        gamepad,
        axis_type: GamepadAxisType::RightStickY,
    };
      // In a real game, the buttons would be configurable, but here we hardcode them
      let thrust = GamepadButton {
        gamepad,
        button_type: GamepadButtonType::LeftTrigger2,
    };
    /*
    let heal_button = GamepadButton {
        gamepad, button_type: GamepadButtonType::East
    };

    if buttons.just_pressed(jump_button) {
        // button just pressed: make the player jump
    }
    */


    for (mut ext_force, mut transform, mut orientation) in query.iter_mut() {
        if let (Some(x), Some(y)) = (axes.get(axis_lx), axes.get(axis_ly)) {
            ext_force.force = Vec2::new(x*SECONDARY_THRUST, y*SECONDARY_THRUST);
            //println!("thrust factor ({},{})N", x, y);
        };

        if buttons.pressed(thrust) {
            ext_force.force = Vec2::new(-orientation.theta.sin()*PRIMARY_THRUST, orientation.theta.cos()*PRIMARY_THRUST);
            //println!("thrust!!")
        }
    

        if let (Some(x), Some(y)) = (axes.get(axis_rx), axes.get(axis_ry)) {

            let acute_angle = if x == 0. {
                // if x is 0 then return to previous state and avoid division by zero
                return;
            } else {
                y.abs() / x.abs().atan()
            };
            let acute_angle = (y.abs() / x.abs()).atan();
            let theta = if y.is_sign_positive() {
                // quadrant I
                if x.is_sign_positive() {
                    acute_angle
                } else {
                    // quadrant II
                    PI - acute_angle
                }
            } else {
                // quadrant III
                if x.is_sign_negative() {
                    PI + acute_angle
                } else {
                    // quadrant IV
                    (2. * PI) - acute_angle
                }
            };
            //println!("{}", theta);
            // account for sprite starting with Pi / 2 rotation
            transform.rotation = Quat::from_rotation_z(theta - (PI / 2.));
            ext_force.torque = 20.;
            orientation.theta = theta - (PI / 2.);

            println!("{}", theta);
        }
    }


}

