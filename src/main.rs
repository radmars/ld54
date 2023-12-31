//! Entrypoint for the game jam
// Turn on some more aggressive warnings from clippy. They shouldn't break the
// build, but should tell you if you're doing something crazy.
#![warn(clippy::pedantic)]
// I hate broken links.
#![deny(rustdoc::broken_intra_doc_links)]
// Bevy passes queries and things by default as values which is a bit hard to
// work around.
#![allow(clippy::needless_pass_by_value)]
// If it turns out we're killing precision we can open these up but they're off
// by default so probably not a big deal
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
// Sadly some systems have super complex type signatures and I'm not sure how to refactor it right now?
#![allow(clippy::type_complexity)]
// Turn on some stuff that isn't in pedantic.
#![warn(
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    rust_2021_compatibility,
    unused,
    missing_docs,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_qualifications,
    variant_size_differences
)]
// I'm not sure i like this 2018 idiom. Can debate it later.
#![allow(elided_lifetimes_in_paths)]

use std::{f32::consts::PI, time::Duration};

use animation::{maybe_change_animation, AnimationIndices};
use bevy::audio::{AudioPlugin, VolumeLevel};
use bevy::{prelude::*, sprite::Anchor, window::WindowResolution};
use bevy_asset_loader::prelude::*;
use bevy_xpbd_2d::prelude::*;
use iyes_progress::prelude::*;
use leafwing_input_manager::prelude::*;
use rand::prelude::*;

const PLAYER_X_SPEED: f32 = 220.0;

const PADDLE_START: Vec3 = Vec3::new(0.0, 270.0, 4.0);
const PADDLE_SIZE: Vec2 = Vec2::new(64.0, 50.0);
const PADDLE_SPEED: f32 = 200.0;

const LEFT_WALL: f32 = -400.0;
const RIGHT_WALL: f32 = 400.0;
const BOTTOM_WALL: f32 = -300.0;
const TOP_WALL: f32 = 300.0;
// We pretend walls are sprites so we can use their collision logic
const WALL_THICKNESS: f32 = 50.0;

const ROCK_WIDTH: f32 = 64.0;
const ROCK_HEIGHT: f32 = 52.0;

const GAP_BETWEEN_PADDLE_AND_TOP: f32 = 60.0;

const GAP_BETWEEN_ROCKS: f32 = 6.0;
const GAP_BETWEEN_ROCKS_AND_BOTTOM: f32 = 30.0;
const GAP_BETWEEN_ROCKS_AND_SIDES: f32 = 30.0;
const GAP_BETWEEN_ROCKS_AND_PADDLE: f32 = 200.0;

const BALL_SPEED: f32 = 250.0;
const BALL_SPAWN_INTERVAL: f32 = 10.0;

const BALL_SOUND_TIME: f32 = 0.169;
const BALL2_SOUND_TIME: f32 = 0.169;
const BREAK_SOUND_TIME: f32 = 0.417;
const EXPLOSION_SOUND_TIME: f32 = 1.878;
const JUMP_SOUND_TIME: f32 = 0.234;
const STEP1_SOUND_TIME: f32 = 0.225;
const STEP2_SOUND_TIME: f32 = 0.225;
const WALL_SOUND_TIME: f32 = 0.139;

mod animation;
mod player;
mod paddle;

#[derive(Resource)]
struct GameOptions {
    debug: bool,
    skip: bool,
}

impl GameOptions {
    #[allow(dead_code)]
    fn insert(&mut self, key: &str, value: &str) {
        match key {
            "debug" => self.debug = ! value.is_empty(),
            "skip"  => self.skip = ! value.is_empty(),
            _ => warn!("Unknown key {}", key)
        }
    }
}

#[derive(States, Default, Copy, Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    #[default]
    Loading,
    Setup,
    Splash,
    Playing,
    GameOver,
}

#[cfg(target_arch = "wasm32")]
fn get_browser_options(game_options: &mut GameOptions) {
    let mut options = HashMap::<String, String>::new();
    console_error_panic_hook::set_once();
    let w = web_sys::window().expect("Couldn't find the window!");
    let s = w.location().search().expect("No search?");
    if let Some(text) = s.get(1..) {
        if text.contains('&') {
            text.split('&')
                .filter_map(|sub| sub.split_once('='))
                .for_each(|(left, right)| {
                    game_options.insert(left, right);
                });
        } else if let Some((left, right)) = text.split_once('=') {
            game_options.insert(left, right);
        }
    }
}

fn main() {
    let mut game_options = GameOptions {
        debug: false,
        skip: false,
    };

    if cfg!(target_arch = "wasm32") {
        #[cfg(target_arch = "wasm32")]
        get_browser_options(&mut game_options);
    }
    else {
        game_options.debug = true;
        game_options.skip = true;
    }

    let mut app = App::default();

    let loading_game_state = GameState::Loading;
    let loading_state = LoadingState::new(loading_game_state);
    let loading_plugin = ProgressPlugin::new(loading_game_state).continue_to(GameState::Setup);

    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    canvas: Some("#bevy".to_owned()),
                    fit_canvas_to_parent: true,
                    focused: true,
                    mode: bevy::window::WindowMode::Windowed,
                    resizable: false,
                    resolution: WindowResolution::new(
                        RIGHT_WALL - LEFT_WALL,
                        TOP_WALL - BOTTOM_WALL,
                    ),
                    transparent: true,
                    ..Default::default()
                }),
                ..default()
            })
            // Fix sprite blur
            .set(ImagePlugin::default_nearest())
            .set(AudioPlugin {
                global_volume: GlobalVolume::new(0.3),
            }),
        loading_plugin,
        InputManagerPlugin::<Action>::default(),
        PhysicsPlugins::default(),
    ))
    .add_loading_state(loading_state)
    .add_collection_to_loading_state::<_, LDAssets>(loading_game_state)
    .add_state::<GameState>()
    .insert_resource(player::PlayerAnimationTable::default())
    .insert_resource(Msaa::Off)
    .insert_resource(ClearColor(Color::hex("#000000").unwrap()))
    .insert_resource(Randomizer::default())
    .insert_resource(if game_options.debug {
        PhysicsDebugConfig::all()
    } else {
        PhysicsDebugConfig::none()
    })
    .insert_resource(game_options)
    .insert_resource(Gravity(Vec2::new(0.0, -800.0)))
    .insert_resource(BallSpawnTimer::default())
    .insert_resource(WalkSoundStatus {
        last_sound: 2,
        time_since_sound: 0.0,
    })
    // .add_systems(Update, bevy::window::close_on_esc)
    .add_systems(Update, (wait_to_start).run_if(in_state(GameState::Splash)))
    .add_systems(
        Update,
        (wait_to_start).run_if(in_state(GameState::GameOver)),
    )
    .add_systems(OnEnter(GameState::Setup), setup)
    .add_systems(OnEnter(GameState::Splash), splash_setup)
    .add_systems(OnEnter(GameState::GameOver), gg_setup)
    .add_systems(OnExit(GameState::GameOver), remove_all_sprites)
    .add_systems(OnExit(GameState::Splash), remove_all_sprites)
    .add_systems(OnExit(GameState::Playing), remove_all_sprites)
    .add_systems(OnExit(GameState::GameOver), remove_all_text)
    .add_systems(OnExit(GameState::Playing), remove_all_text)
    .add_systems(OnEnter(GameState::Playing), playing_setup)
    .add_systems(
        Update,
        (player_inputs, animation::animate).run_if(in_state(GameState::Playing)),
    )
    .add_systems(Update, ball_collisions.run_if(in_state(GameState::Playing)))
    .add_systems(
        Update,
        (
            player_animation,
            paddle_ai,
            check_for_gg,
            spawn_ball_timer,
            kill_timed_audio,
            update_timer,
        )
            .run_if(in_state(GameState::Playing)),
    )
    .run();
}

#[derive(Resource)]
struct Randomizer {
    rng: SmallRng,
}

impl Default for Randomizer {
    fn default() -> Self {
        Randomizer {
            rng: SmallRng::from_entropy(),
        }
    }
}

#[derive(AssetCollection, Resource)]
struct LDAssets {
    #[asset(path = "FiraSans-Bold.ttf")]
    font: Handle<Font>,

    #[asset(texture_atlas(tile_size_x = 64., tile_size_y = 70., columns = 9, rows = 1))]
    #[asset(path = "player.png")]
    player: Handle<TextureAtlas>,

    #[asset(texture_atlas(tile_size_x = 64.0, tile_size_y = 64.0, columns = 1, rows = 2))]
    #[asset(path = "rocks.png")]
    rocks: Handle<TextureAtlas>,

    #[asset(path = "paddle.png")]
    paddle: Handle<Image>,

    #[asset(path = "gameover.png")]
    gameover: Handle<Image>,

    #[asset(path = "gamebg.png")]
    gamebg: Handle<Image>,

    #[asset(path = "splash.png")]
    splash: Handle<Image>,

    #[asset(path = "bomb.png")]
    bomb: Handle<Image>,

    #[asset(path = "audio/ball.ogg")]
    ball_sound: Handle<AudioSource>,

    #[asset(path = "audio/ball2.ogg")]
    ball2_sound: Handle<AudioSource>,

    #[asset(path = "audio/break.ogg")]
    break_sound: Handle<AudioSource>,

    #[asset(path = "audio/explosion.ogg")]
    explosion_sound: Handle<AudioSource>,

    #[asset(path = "audio/jump.ogg")]
    jump_sound: Handle<AudioSource>,

    #[asset(path = "audio/step1.ogg")]
    step1_sound: Handle<AudioSource>,

    #[asset(path = "audio/step2.ogg")]
    step2_sound: Handle<AudioSource>,

    #[asset(path = "audio/wall.ogg")]
    wall_sound: Handle<AudioSource>,

    #[asset(path = "audio/ld54-main.ogg")]
    bgm: Handle<AudioSource>,
}

fn setup(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    config: Res<GameOptions>,
) {
    commands.spawn(Camera2dBundle::default());

    if config.skip {
        next_state.set(GameState::Playing);
    } else {
        next_state.set(GameState::Splash);
    }
}

fn splash_setup(assets: Res<LDAssets>, mut commands: Commands) {
    commands.spawn(SpriteBundle {
        texture: assets.splash.clone(),
        ..default()
    });
}

fn gg_setup(assets: Res<LDAssets>, mut commands: Commands) {
    commands.spawn(SpriteBundle {
        texture: assets.gameover.clone(),
        ..default()
    });
    let text_style = TextStyle {
        font: assets.font.clone(),
        font_size: 60.0,
        color: Color::BLACK,
    };
    // TODO: Put this in the middle of the screen and blink.
    commands.spawn(
        TextBundle::from_section("Press space to restart", text_style)
            .with_text_alignment(TextAlignment::Center)
            .with_style(Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(5.0),
                right: Val::Px(15.0),
                ..default()
            }),
    );
}

fn remove_all_text(mut commands: Commands, things_to_remove: Query<Entity, With<Text>>) {
    for thing_to_remove in &things_to_remove {
        let mut entity_commands = commands.entity(thing_to_remove);
        entity_commands.despawn();
    }
}

fn remove_all_sprites(
    mut commands: Commands,
    things_to_remove: Query<Entity, Or<(With<Sprite>, With<TextureAtlasSprite>)>>,
) {
    for thing_to_remove in &things_to_remove {
        let mut entity_commands = commands.entity(thing_to_remove);
        entity_commands.despawn();
    }
}

fn wait_to_start(k: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if k.just_pressed(KeyCode::Space) {
        next_state.set(GameState::Playing);
    }
}

fn paddle_ai(
    time: Res<Time>,
    mut paddle_query: Query<(&mut paddle::Paddle, &mut Transform), Without<Ball>>,
    ball_query: Query<(&Ball, &Transform, &LinearVelocity), Without<paddle::Paddle>>,
) {
    let Ok((mut paddle, mut paddle_transform)) = paddle_query.get_single_mut() else {
        return;
    };

    // Try to catch the ball that will soonest collide with the top
    let result = ball_query
        .iter()
        // Unwrap the translation
        .map(|(_, t, v)| (t.translation, v))
        // Ignore balls that are above the paddle
        .filter(|(t, _)| t.y < (PADDLE_START.y - PADDLE_SIZE.y / 2.0))
        .map(|(t, v)| ((TOP_WALL - t.y) / v.y, t))
        // Correct for balls with downward velocities
        .map(|(i, t)| {
            if i < 0.0 {
                (-i + (TOP_WALL - BOTTOM_WALL) / 2.0, t)
            } else {
                (i, t)
            }
        })
        .min_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .expect("Encountered a bad floating point!")
        })
        .map(|(_, translation)| translation);

    let closest: Vec3;
    if let Some(i) = result {
        closest = i;
    } else {
        return;
    }

    let amount = PADDLE_SPEED * time.delta().as_secs_f32();

    paddle.left = paddle_transform.translation.x > closest.x;

    if paddle.left {
        if paddle_transform.translation.x - PADDLE_SIZE.x / 2. > LEFT_WALL {
            paddle_transform.translation.x -= amount;
        }
    } else if paddle_transform.translation.x + PADDLE_SIZE.x / 2. < RIGHT_WALL {
        paddle_transform.translation.x += amount;
    }
}

fn playing_setup(
    assets: Res<LDAssets>,
    mut rng: ResMut<Randomizer>,
    mut commands: Commands,
    player_animations: Res<player::PlayerAnimationTable>,
) {
    let paddle_y = TOP_WALL - GAP_BETWEEN_PADDLE_AND_TOP - PADDLE_SIZE.y;
    commands.spawn(SpriteBundle {
        texture: assets.gamebg.clone(),
        ..default()
    });
    commands.spawn((
        AudioBundle {
            source: assets.bgm.clone(),
            settings: PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: bevy::audio::Volume::Absolute(VolumeLevel::new(0.3)),
                ..Default::default()
            },
        },
        SpriteBundle::default(),
    ));
    commands
        .spawn(WallBundle::new(WallLocation::Left, false));
    commands
        .spawn(WallBundle::new(WallLocation::Right, false));
    commands
        .spawn(WallBundle::new(WallLocation::Bottom, false));
    commands
        .spawn(WallBundle::new(WallLocation::Top, true));

    commands.spawn(paddle::PaddleBundle::new(&assets));

    commands.spawn(player::PlayerBundle::new(&assets, &player_animations));

    commands.spawn(BallBundle::new(&assets, &mut rng, PADDLE_START));

    spawn_rocks(paddle_y, rng, &mut commands, &assets);

    let text_style = TextStyle {
        font: assets.font.clone(),
        font_size: 30.0,
        color: Color::WHITE,
    };
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new("Gods' wrath endured for: ", text_style.clone()),
            TextSection::from_style(text_style.clone()),
        ])
        .with_text_alignment(TextAlignment::Left)
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        }),
        SurvivalTime(0.0),
    ));
}

fn spawn_rocks(paddle_y: f32, mut rng: ResMut<'_, Randomizer>, commands: &mut Commands<'_, '_>, assets: &Res<'_, LDAssets>) {
    // Spawn as many rocks as we can given the boundaries defined by the constants
    let total_width_of_rocks = (RIGHT_WALL - LEFT_WALL) - 2. * GAP_BETWEEN_ROCKS_AND_SIDES;
    let top_edge_of_rocks = paddle_y - GAP_BETWEEN_ROCKS_AND_PADDLE;
    let bottom_edge_of_rocks = BOTTOM_WALL + GAP_BETWEEN_ROCKS_AND_BOTTOM;
    let total_height_of_rocks = top_edge_of_rocks - bottom_edge_of_rocks;

    assert!(total_width_of_rocks > 0.0);
    assert!(total_height_of_rocks > 0.0);

    // Given the space available, compute how many rows and columns of bricks we can fit
    let n_columns = (total_width_of_rocks / (ROCK_WIDTH + GAP_BETWEEN_ROCKS)).floor() as usize;
    let n_rows = (total_height_of_rocks / (ROCK_HEIGHT + GAP_BETWEEN_ROCKS)).floor() as usize;
    let n_vertical_gaps = n_columns - 1;

    // Because we need to round the number of columns,
    // the space on the top and sides of the rocks only captures a lower bound, not an exact value
    let center_of_rocks = (LEFT_WALL + RIGHT_WALL) / 2.0;
    let left_edge_of_rocks = center_of_rocks
        // Space taken up by the bricks
        - (n_columns as f32 / 2.0 * ROCK_WIDTH)
        // Space taken up by the gaps
        - n_vertical_gaps as f32 / 2.0 * GAP_BETWEEN_ROCKS;

    // In Bevy, the `translation` of an entity describes the center point,
    // not its bottom-left corner
    let offset_x = left_edge_of_rocks + ROCK_WIDTH / 2.0;
    let offset_y = bottom_edge_of_rocks + ROCK_HEIGHT / 2.0;

    let image_indices: [usize; 2] = [0, 1];

    for row in 0..n_rows {
        for column in 0..n_columns {
            let rock_position = Vec2::new(
                offset_x + column as f32 * (ROCK_WIDTH + GAP_BETWEEN_ROCKS),
                offset_y + row as f32 * (ROCK_HEIGHT + GAP_BETWEEN_ROCKS),
            );

            let image_index = image_indices.choose(&mut rng.rng).unwrap();
            commands
                .spawn(RockBundle::new(assets, *image_index, rock_position));
        }
    }
}


#[derive(Resource)]
struct WalkSoundStatus {
    last_sound: i8,
    time_since_sound: f32,
}

#[derive(Component)]
struct SurvivalTime(f32);

// Define the collision layers
#[derive(PhysicsLayer)]
enum Layer {
    Ball,
    Rock,
    Player,
    Wall,
    Paddle,
}

#[derive(Component, Default)]
struct Rock;

#[derive(Bundle)]
struct RockBundle {
    rock: Rock,
    #[bundle()]
    sprite: SpriteSheetBundle,
    collider: Collider,
    rigid_body: RigidBody,
    collision_layer: CollisionLayers,
    sleeping_disabled: SleepingDisabled,
}

#[derive(Bundle)]
struct TimedAudioBundle {
    #[bundle()]
    audio_bundle: AudioBundle,
    timed_audio: TimedAudio,
}

#[derive(Component)]
struct TimedAudio {
    timer: Timer,
}

impl RockBundle {
    fn new(assets: &Res<LDAssets>, image_index: usize, rock_position: Vec2) -> RockBundle {
        RockBundle {
            sprite: SpriteSheetBundle {
                texture_atlas: assets.rocks.clone(),
                sprite: TextureAtlasSprite {
                    index: image_index,
                    ..default()
                },
                transform: Transform {
                    translation: rock_position.extend(1.0),
                    ..default()
                },
                ..default()
            },
            rock: Rock,
            // collider: Collider::cuboid(60.0, if image_index == 1 { 18.0 } else { 25.0 }),
            rigid_body: RigidBody::Static,
            collision_layer: CollisionLayers::new([Layer::Rock], [Layer::Ball, Layer::Player]),
            sleeping_disabled: SleepingDisabled,
            collider: Collider::capsule_endpoints(
                Vec2::new(-20.0, 0.0),
                Vec2::new(20.0, 0.0),
                if image_index == 1 { 13.0 } else { 15.0 },
            ),

        }
    }
}

#[derive(Component, Default)]
struct Ball;

#[derive(Bundle)]
struct BallBundle {
    ball: Ball,
    #[bundle()]
    sprite: SpriteBundle,
    collider: Collider,
    rigid_body: RigidBody,
    linear_velocity: LinearVelocity,
    restitution: Restitution,
    friction: Friction,
    gravity_scale: GravityScale,
    collision_layer: CollisionLayers,
    sleeping_disabled: SleepingDisabled,
}

impl BallBundle {
    fn new(assets: &Res<LDAssets>, rng: &mut Randomizer, paddle_location: Vec3) -> BallBundle {
        // Randomize starting direction of ball
        let angle = rng.rng.gen_range(-PI / 4.0..PI / 4.0);
        let rotation = Quat::from_axis_angle(Vec3::Z, angle);
        let start_velocity = rotation.mul_vec3(Vec3::new(0., -BALL_SPEED, 0.)).truncate();

        BallBundle {
            ball: Ball,
            sprite: SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::Custom(Vec2::new(0.0, -0.1)),
                    ..Default::default()
                },
                texture: assets.bomb.clone(),
                transform: Transform::from_translation(
                    paddle_location + Vec3::new(0., (-PADDLE_SIZE.y / 2.) - 10., 0.),
                ),
                ..default()
            },
            rigid_body: RigidBody::Dynamic,
            collider: Collider::ball(10.0),
            linear_velocity: LinearVelocity(start_velocity),
            // external_force: ExternalForce::new(start_velocity * 10000.0).with_persistence(false),
            restitution: Restitution::new(1.0).with_combine_rule(CoefficientCombine::Max),
            friction: Friction::ZERO,
            gravity_scale: GravityScale(0.0),
            collision_layer: CollisionLayers::new(
                [Layer::Ball],
                [
                    Layer::Rock,
                    Layer::Player,
                    Layer::Paddle,
                    Layer::Wall,
                    Layer::Ball,
                ],
            ),
            sleeping_disabled: SleepingDisabled,
        }
    }
}

#[derive(Resource)]
struct BallSpawnTimer(Timer);

impl Default for BallSpawnTimer {
    fn default() -> Self {
        BallSpawnTimer(Timer::new(
            Duration::from_secs_f32(BALL_SPAWN_INTERVAL),
            TimerMode::Repeating,
        ))
    }
}

fn spawn_ball_timer(
    time: Res<Time>,
    assets: Res<LDAssets>,
    mut rng: ResMut<Randomizer>,
    mut ball_timer: ResMut<BallSpawnTimer>,
    mut commands: Commands,
    paddle: Query<&Transform, With<paddle::Paddle>>,
) {
    let Ok(paddle_xform) = paddle.get_single() else {
        return;
    };

    ball_timer.0.tick(time.delta());

    if ball_timer.0.just_finished() {
        commands.spawn(BallBundle::new(&assets, &mut rng, paddle_xform.translation));
    }
}

enum WallLocation {
    Left,
    Right,
    Bottom,
    Top,
}

impl WallLocation {
    fn position(&self) -> Vec2 {
        match self {
            WallLocation::Left => Vec2::new(LEFT_WALL - WALL_THICKNESS / 2., 0.),
            WallLocation::Right => Vec2::new(RIGHT_WALL + WALL_THICKNESS / 2., 0.),
            WallLocation::Bottom => Vec2::new(0., BOTTOM_WALL - WALL_THICKNESS / 2.),
            WallLocation::Top => Vec2::new(0., TOP_WALL + WALL_THICKNESS / 2.),
        }
    }

    fn size(&self) -> Vec2 {
        let arena_height = TOP_WALL - BOTTOM_WALL;
        let arena_width = RIGHT_WALL - LEFT_WALL;
        // Make sure we haven't messed up our constants
        assert!(arena_height > 0.0);
        assert!(arena_width > 0.0);

        match self {
            WallLocation::Left | WallLocation::Right => {
                Vec2::new(WALL_THICKNESS, arena_height + WALL_THICKNESS)
            }
            WallLocation::Bottom | WallLocation::Top => {
                Vec2::new(arena_width + WALL_THICKNESS, WALL_THICKNESS)
            }
        }
    }
}

#[derive(Component)]
struct Wall {
    ball_destroyer: bool,
}

#[derive(Bundle)]
struct WallBundle {
    wall: Wall,
    sprite_bundle: SpriteBundle,
    collider: Collider,
    rigid_body: RigidBody,
    restitution: Restitution,
    collision_layer: CollisionLayers,
}

impl WallBundle {
    // This "builder method" allows us to reuse logic across our wall entities,
    // making our code easier to read and less prone to bugs when we change the logic
    fn new(location: WallLocation, ball_destroyer: bool) -> WallBundle {
        WallBundle {
            wall: Wall { ball_destroyer },
            rigid_body: RigidBody::Static,
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    // We need to convert our Vec2 into a Vec3, by giving it a z-coordinate
                    // This is used to determine the order of our sprites
                    translation: location.position().extend(0.0),
                    // The z-scale of 2D objects must always be 1.0,
                    // or their ordering will be affected in surprising ways.
                    // See https://github.com/bevyengine/bevy/issues/4149
                    scale: location.size().extend(1.0),
                    ..default()
                },
                sprite: Sprite { ..default() },
                ..default()
            },
            collider: Collider::cuboid(location.size().x, location.size().y),
            restitution: Restitution::new(1.0).with_combine_rule(CoefficientCombine::Max),
            collision_layer: CollisionLayers::new(
                [Layer::Wall],
                [Layer::Ball, Layer::Player, Layer::Rock],
            ),
        }
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
enum Action {
    Move,
    Jump,
}


fn player_inputs(
    mut player_query: Query<(&mut LinearVelocity, &ActionState<Action>), With<player::Player>>,
    mut commands: Commands,
    assets: Res<LDAssets>,
    mut walk_sound_status: ResMut<WalkSoundStatus>,
    time: Res<Time>,
) {
    let Ok((mut velocity, action_state)) = player_query.get_single_mut() else {
        return;
    };

    if action_state.pressed(Action::Move) {
        let x_amount = action_state.clamped_value(Action::Move);
        velocity.x = x_amount * PLAYER_X_SPEED;

        if walk_sound_status.time_since_sound > 0.25 && velocity.y.abs() < 0.05 {
            walk_sound_status.time_since_sound = 0.0;
            if walk_sound_status.last_sound == 1 {
                play_audio(assets.step2_sound.clone(), &mut commands, STEP2_SOUND_TIME);
                walk_sound_status.last_sound = 2;
            } else {
                play_audio(assets.step1_sound.clone(), &mut commands, STEP1_SOUND_TIME);
                walk_sound_status.last_sound = 1;
            }
        } else {
            walk_sound_status.time_since_sound += time.delta_seconds();
        }
    }

    if action_state.just_pressed(Action::Jump) {
        // THIS IS NOT THE CORRECT WAY TO DO IT, SOLEN FROM:
        // https://github.com/Jondolf/bevy_xpbd/blob/8b2ea8fd4754fb3ecd51f79fad282d22631d2c7f/crates/bevy_xpbd_2d/examples/one_way_platform_2d.rs#L152-L157
        if velocity.y.abs() < 0.5 {
            velocity.y = 400f32;
            play_audio(assets.jump_sound.clone(), &mut commands, JUMP_SOUND_TIME);
        }
    }
}

fn check_for_gg(
    player_xform: Query<&Transform, With<player::Player>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    assets: Res<LDAssets>,
) {
    let Ok(player_xform) = player_xform.get_single() else {
        return;
    };

    if player_xform.translation.y < -270.0 {
        play_audio(
            assets.explosion_sound.clone(),
            &mut commands,
            EXPLOSION_SOUND_TIME,
        );
        next_state.set(GameState::GameOver);
    }
}

pub(crate) fn player_animation(
    mut player_query: Query<
        (
            &LinearVelocity,
            &mut TextureAtlasSprite,
            &mut AnimationIndices,
        ),
        With<player::Player>,
    >,
    player_animations: Res<player::PlayerAnimationTable>,
) {
    let Ok((velocity, mut atlas, mut animation)) = player_query.get_single_mut() else {
        return;
    };

    // Priority is dealing with jump, then dealing with walk.

    if velocity.y.abs() > 0.2 {
        if velocity.y < 0.0 {
            maybe_change_animation(&mut animation, &player_animations.jump_down);
        } else {
            maybe_change_animation(&mut animation, &player_animations.jump_up);
        }
    } else if velocity.x.abs() > 0.2 {
        maybe_change_animation(&mut animation, &player_animations.walk);
    } else {
        maybe_change_animation(&mut animation, &player_animations.idle);
    }

    if velocity.x.abs() > 0.2 {
        atlas.flip_x = velocity.x < 0.0;
    }
}

fn ball_collisions(
    mut commands: Commands,
    mut collision_end: EventReader<CollisionEnded>,
    mut rng: ResMut<Randomizer>,
    balls: Query<Entity, With<Ball>>,
    collisions: Query<
        (
            Entity,
            Option<&Rock>,
            Option<&Wall>,
            Option<&player::Player>,
            Option<&paddle::Paddle>,
        ),
        With<Collider>,
    >,
    assets: Res<LDAssets>,
) {
    for e in &mut collision_end {
        let maybe_ball = balls.get(e.0).ok().or_else(|| balls.get(e.1).ok());

        if let Some(ball) = maybe_ball {
            if let Some((
                target,
                maybe_rock,
                maybe_wall,
                maybe_player,
                maybe_paddle,
            )) = collisions
                .get(e.0)
                .ok()
                .or_else(|| collisions.get(e.1).ok())
            {
                if maybe_rock.is_some() {
                    commands.entity(target).despawn_recursive();
                    play_audio(assets.break_sound.clone(), &mut commands, BREAK_SOUND_TIME);
                }

                if let Some(wall) = maybe_wall {
                    if wall.ball_destroyer {
                        commands.entity(ball).despawn_recursive();
                    } else {
                        play_audio(assets.wall_sound.clone(), &mut commands, WALL_SOUND_TIME);
                    }
                }

                if maybe_player.is_some() {
                    let i = rng.rng.gen_range(0..2);
                    if i == 1 {
                        play_audio(assets.ball_sound.clone(), &mut commands, BALL_SOUND_TIME);
                    } else {
                        play_audio(assets.ball2_sound.clone(), &mut commands, BALL2_SOUND_TIME);
                    }
                }

                if maybe_paddle.is_some() {
                    play_audio(assets.ball2_sound.clone(), &mut commands, BALL2_SOUND_TIME);
                }
            }
        }
    }
}

fn kill_timed_audio(
    time: Res<Time>,
    mut query: Query<(Entity, &mut TimedAudio)>,
    mut commands: Commands,
) {
    for mut audio in &mut query {
        audio.1.timer.tick(time.delta());
        if audio.1.timer.just_finished() {
            commands.entity(audio.0).despawn();
        }
    }
}
fn play_audio(source: Handle<AudioSource>, commands: &mut Commands, length: f32) {
    commands.spawn(TimedAudioBundle {
        audio_bundle: AudioBundle {
            source,
            ..default()
        },
        timed_audio: TimedAudio {
            timer: Timer::new(Duration::from_secs_f32(length), TimerMode::Once),
        },
    });
}

fn update_timer(time: Res<Time>, mut text_widget: Query<(&mut Text, &mut SurvivalTime)>) {
    let Ok((mut text, mut survival_time)) = text_widget.get_single_mut() else {
        return;
    };

    survival_time.0 += time.delta_seconds();
    text.sections[1].value = format!("{:.2} s", survival_time.0);
}
