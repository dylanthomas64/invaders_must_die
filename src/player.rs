use crate::{
    components::{FromPlayer, Movable, Player, SpriteSize, Velocity, Laser},
    GameTextures, WinSize, BASE_SPEED, PLAYER_LASER_SIZE, PLAYER_SIZE, SPRITE_SCALE, TIME_STEP, PlayerState, PLAYER_RESPAWN_DELAY,
};
use bevy::{prelude::*, core::FixedTimestep};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(PlayerState::default())
            .add_system_set(
            SystemSet::new()
            .with_run_criteria(FixedTimestep::step(0.5))
            .with_system(player_spawn_system)
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
    let now = time.seconds_since_startup();
    let last_shot = player_state.last_shot;

    if !player_state.on && (last_shot == -1. || now > last_shot + PLAYER_RESPAWN_DELAY) {
        // show player score
        println!("Final score: {}", player_state.score);
        player_state.score = 0;

         // add player
    let bottom = -win_size.h / 2.;
    commands
        .spawn_bundle(SpriteBundle {
            texture: game_textures.player.clone(),
            transform: Transform {
                // vec3::new(x, y (+ padding), z)
                translation: Vec3::new(0., bottom + PLAYER_SIZE.1 / 2. * SPRITE_SCALE + 5., 10.),
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
        .insert(Movable {
            auto_despawn: false,
        });
        player_state.spawned();
    }
}

fn player_keyboard_event_system(
    kb: Res<Input<KeyCode>>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    if let Ok(mut velocity) = query.get_single_mut() {
        velocity.x = if kb.pressed(KeyCode::Left) {
            -1.
        } else if kb.pressed(KeyCode::Right) {
            1.
        } else {
            0.
        }
    }
}

fn player_fire_system(
    mut commands: Commands,
    kb: Res<Input<KeyCode>>,
    game_textures: Res<GameTextures>,
    query: Query<&Transform, With<Player>>,
) {
    if let Ok(player_tf) = query.get_single() {
        if kb.just_pressed(KeyCode::Space) {
            let (x, y) = (player_tf.translation.x, player_tf.translation.y);

            // offset to change where laser fires from
            let x_offset = PLAYER_SIZE.0 / 4. * SPRITE_SCALE;
            let y_offset: f32 = 20.;

            // create closure so multiple lasers can be spawned
            let mut spawn_laser = |x_offset: f32, y_offset: f32| {
                commands
                    .spawn_bundle(SpriteBundle {
                        texture: game_textures.player_laser.clone(),
                        transform: Transform {
                            translation: Vec3::new(x + x_offset, y + y_offset, 0.),
                            scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .insert(Laser)
                    .insert(Movable { auto_despawn: true })
                    .insert(Velocity { x: 0., y: 2. })
                    .insert(FromPlayer)
                    .insert(SpriteSize::from(PLAYER_LASER_SIZE));
            };
            spawn_laser(x_offset, 0.);
            spawn_laser(-x_offset, 0.);
            spawn_laser(0., y_offset);
        }
    }
}
