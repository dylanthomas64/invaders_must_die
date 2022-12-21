use std::f32::consts::PI;

use crate::{
    components::{FromPlayer, Laser, Movable, Player, SpriteSize, Velocity, Orientation},
    GameTextures, PlayerState, WinSize, PLAYER_LASER_SIZE, PLAYER_RESPAWN_DELAY, PLAYER_SIZE,
    SPRITE_SCALE, MyGamepad, LASER_VELOCITY,
};
use bevy::{prelude::*, time::FixedTimestep};
use bevy_rapier2d::prelude::{RigidBody, Collider, ExternalForce, Restitution, ReadMassProperties, MassProperties, ColliderMassProperties, ExternalImpulse};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerState::default())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(0.5))
                    .with_system(player_spawn_system),
            )
            .add_system(player_keyboard_event_system)
            .add_system(player_fire_system);
    }
}

fn player_spawn_system(
    mut commands: Commands,
    mut player_state: ResMut<PlayerState>,
    time: Res<Time>,
    game_textures: Res<GameTextures>,
    win_size: Res<WinSize>,
) {
    let now = time.elapsed_seconds_f64();
    let last_shot = player_state.last_shot;

    if !player_state.on && (last_shot == -1. || now > last_shot + PLAYER_RESPAWN_DELAY) {
        // reset score
        player_state.score = 0;

        // add player
        let bottom = -win_size.h / 2.;
        commands
            .spawn(SpriteBundle {
                texture: game_textures.player.clone(),
                transform: Transform {
                    // vec3::new(x, y (+ padding), z)
                    translation: Vec3::new(
                        0.,
                        bottom + PLAYER_SIZE.1 / 2. * SPRITE_SCALE + 5.,
                        10.,
                    ),
                    scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
                    ..Default::default()
                },

                /* // add rectangle
                sprite: Sprite {
                    color: Color::rgb(0.25, 0.25, 0.75),
                    custom_size: Some(Vec2::new(150.0, 150.0)),
                    ..Default::default()
                }, */
                ..Default::default()
            })
            .insert(Player)
            .insert(SpriteSize::from(PLAYER_SIZE))
            .insert(Velocity { x: 0., y: 0. })
            .insert(Orientation::default())
            .insert(Movable {
                auto_despawn: false,
            })
            // PLAYER PHYICS
            .insert(RigidBody::Dynamic)
            .insert(Collider::triangle(Vec2::new(-50.0, -30.0), Vec2::new(50.0, -30.0), Vec2::new(0.0, 40.0)))
            .insert(Restitution::coefficient(0.7))
            .insert(ExternalForce {
                force: Vec2::new(0., 0.),
                torque: 0.
            });
            player_state.spawned();
    }
}

fn player_keyboard_event_system(
    kb: Res<Input<KeyCode>>,
    mut query: Query<(&mut Velocity, &mut Orientation), With<Player>>,
) {
    for (mut velocity, mut orientation) in query.iter_mut() {

        // WASD
        velocity.x = if kb.pressed(KeyCode::A) {
            -1.
        } else if kb.pressed(KeyCode::D) {
            1.
        } else {
            0.
        };

        velocity.y = if kb.pressed(KeyCode::W) {
            1.
        } else if kb.pressed(KeyCode::S) {
            -1.
        } else {
            0.
        };

        // DIRECTION w/ arrow keys
        orientation.theta = if kb.pressed(KeyCode::Up) {
            0.
        } else if kb.pressed(KeyCode::Down) {
            PI
        } else if kb.pressed(KeyCode::Right) {
            3. * PI / 2.
        } else if kb.pressed(KeyCode::Left) {
            PI / 2.
        }
        else { continue };



    }
}

fn player_fire_system(
    mut commands: Commands,
    kb: Res<Input<KeyCode>>,
    button: Res<Input<GamepadButton>>,
    my_gamepad: Option<Res<MyGamepad>>,
    game_textures: Res<GameTextures>,
    query: Query<(&Transform, &Orientation), With<Player>>,
) {

    // locate connected game pad
    let gamepad = if let Some(gp) = my_gamepad {
        // a gamepad is connected, we have the id
        gp.0
    } else {
        // no gamepad is connected
        return;
    };

    for (player_tf, orientation) in query.iter() {
        if kb.just_pressed(KeyCode::Space) || button.pressed(GamepadButton {
            gamepad, button_type: GamepadButtonType::RightTrigger2
        }) {
            let (x, y, theta) = (player_tf.translation.x, player_tf.translation.y, orientation.theta);

            // offset to change where laser fires from
            let x_offset = PLAYER_SIZE.0 / 4. * SPRITE_SCALE;
            let y_offset: f32 = 20.;

            // create closure so multiple lasers can be spawned
            let mut spawn_laser = |x_offset: f32, y_offset: f32| {
                commands
                    .spawn(SpriteBundle {
                        texture: game_textures.player_laser.clone(),
                        transform: Transform {
                            translation: Vec3::new(x + x_offset, y + y_offset, 0.),
                            rotation: Quat::from_rotation_z(theta),
                            scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .insert(Laser)
                    .insert(ExternalImpulse {
                        impulse: Vec2::new(- 2.*orientation.theta.sin()*LASER_VELOCITY, 2.*orientation.theta.cos()*LASER_VELOCITY),
                        torque_impulse: 0.,
                    })
                    //.insert(Velocity { x: - 2.*orientation.theta.sin(), y: 2.*orientation.theta.cos() }) // laser speed of 2
                    .insert(FromPlayer)
                    .insert(SpriteSize::from(PLAYER_LASER_SIZE))
                    .insert(Orientation { theta: theta })
                    .insert(RigidBody::Dynamic)
                    .insert(Collider::cuboid(1., 2.))
                    .insert(Restitution::coefficient(0.0))
                    .insert(ReadMassProperties(MassProperties {
                        ..Default::default()
                    }));
                    //.insert(ColliderMassProperties::Density(0.01));
            };

            spawn_laser(0., 50.);

            // spawn three lasers
            //spawn_laser(x_offset, 0.);
            //spawn_laser(-x_offset, 0.);
            //spawn_laser(0., y_offset);
        }
    }
}
