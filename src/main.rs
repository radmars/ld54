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

use std::f32::consts::PI;
use std::time::Duration;

use animation::{maybe_change_animation, AnimationIndices};
use bevy::{prelude::*, sprite::Anchor, window::WindowResolution};
use bevy_asset_loader::prelude::*;
use bevy_xpbd_2d::prelude::*;
use iyes_progress::prelude::*;
use leafwing_input_manager::{axislike::VirtualAxis, prelude::*};
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

mod animation;

#[derive(Resource)]
struct PlayerAnimationTable {
    idle: AnimationIndices,
    walk: AnimationIndices,
    jump_up: AnimationIndices,
    jump_down: AnimationIndices,
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

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

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
            .set(ImagePlugin::default_nearest()),
        loading_plugin,
        InputManagerPlugin::<Action>::default(),
        PhysicsPlugins::default(),
    ))
    .add_loading_state(loading_state)
    .add_collection_to_loading_state::<_, LDAssets>(loading_game_state)
    .add_state::<GameState>()
    .insert_resource(PhysicsTimestep::Fixed(1.0 / 120.0))
    .insert_resource(PlayerAnimationTable {
        idle: AnimationIndices {
            first: 0,
            last: 0,
            timer: Timer::from_seconds(0.03, TimerMode::Repeating),
        },
        walk: AnimationIndices {
            first: 1,
            last: 2,
            timer: Timer::from_seconds(0.03, TimerMode::Repeating),
        },
        jump_up: AnimationIndices {
            first: 4,
            last: 5,
            timer: Timer::from_seconds(0.03, TimerMode::Once),
        },
        jump_down: AnimationIndices {
            first: 6,
            last: 6,
            timer: Timer::from_seconds(0.03, TimerMode::Once),
        },
    })
    .insert_resource(Msaa::Off)
    .insert_resource(ClearColor(Color::hex("#000000").unwrap()))
    .insert_resource(Randomizer::default())
    .insert_resource(PhysicsDebugConfig::all())
    .insert_resource(Gravity(Vec2::new(0.0, -800.0)))
    .insert_resource(SubstepCount(30))
    .insert_resource(BallSpawnTimer::default())
    .add_systems(Update, bevy::window::close_on_esc)
    .add_systems(Update, (wait_to_start).run_if(in_state(GameState::Splash)))
    .add_systems(OnEnter(GameState::Setup), setup)
    .add_systems(OnEnter(GameState::Splash), splash_setup)
    .add_systems(OnEnter(GameState::GameOver), gg_setup)
    .add_systems(OnExit(GameState::Splash), remove_all_sprites)
    .add_systems(OnExit(GameState::Playing), remove_all_sprites)
    .add_systems(OnEnter(GameState::Playing), playing_setup)
    .add_systems(
        Update,
        (player_inputs, animation::animate).run_if(in_state(GameState::Playing)),
    )
    .add_systems(Update, ball_collisions.run_if(in_state(GameState::Playing)))
    .add_systems(
        Update,
        (player_animation, paddle_ai, check_for_gg, spawn_ball)
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
}

fn setup(mut commands: Commands, mut next_state: ResMut<NextState<GameState>>) {
    commands.spawn(Camera2dBundle::default());
    next_state.set(GameState::Playing);
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
    if k.just_pressed(KeyCode::J) {
        next_state.set(GameState::Playing);
    }
}

fn spawn_paddle(commands: &mut Commands, assets: &Res<LDAssets>) {
    commands.spawn(PaddleBundle {
        paddle: Paddle { left: true },
        sprite: SpriteBundle {
            texture: assets.paddle.clone(),
            transform: Transform::from_translation(PADDLE_START),
            ..Default::default()
        },
        collider: Collider::capsule_endpoints(Vec2::new(-11.0, -8.0), Vec2::new(11.0, -8.0), 15.0),
        rigid_body: RigidBody::Static,
        restitution: Restitution::new(1.0).with_combine_rule(CoefficientCombine::Max),
    });
}

fn paddle_ai(
    time: Res<Time>,
    mut paddle_query: Query<(&mut Paddle, &mut Transform), Without<Ball>>,
    mut ball_query: Query<(&Ball, &Transform), Without<Paddle>>,
) {
    let Ok((mut paddle, mut transform)) = paddle_query.get_single_mut() else {
        return;
    };
    let Ok((_ball, ball_transform)) = ball_query.get_single_mut() else {
        return;
    };

    let amount = PADDLE_SPEED * time.delta().as_secs_f32();

    paddle.left = transform.translation.x > ball_transform.translation.x;

    if paddle.left {
        if transform.translation.x - PADDLE_SIZE.x / 2. > LEFT_WALL {
            transform.translation.x -= amount;
        }
    } else if transform.translation.x + PADDLE_SIZE.x / 2. < RIGHT_WALL {
        transform.translation.x += amount;
    }
}

fn playing_setup(
    assets: Res<LDAssets>,
    mut rng: ResMut<Randomizer>,
    mut commands: Commands,
    player_animations: Res<PlayerAnimationTable>,
) {
    let paddle_y = TOP_WALL - GAP_BETWEEN_PADDLE_AND_TOP - PADDLE_SIZE.y;
    commands.spawn(SpriteBundle {
        texture: assets.gamebg.clone(),
        ..default()
    });

    commands.spawn(WallBundle::new(WallLocation::Left));
    commands.spawn(WallBundle::new(WallLocation::Right));
    commands.spawn(WallBundle::new(WallLocation::Bottom));
    commands.spawn(WallBundle::new(WallLocation::Top));

    spawn_paddle(&mut commands, &assets);

    let idle_player = player_animations.idle.clone();

    let pb = PlayerBundle {
        sprite: SpriteSheetBundle {
            texture_atlas: assets.player.clone(),
            sprite: TextureAtlasSprite {
                index: idle_player.first,
                anchor: Anchor::Custom(Vec2::new(-0.1, -0.2)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 100.0, 1.0)),
            ..default()
        },
        input_manager: InputManagerBundle::<Action> {
            input_map: player_input_map(),
            ..default()
        },
        animation_indices: idle_player,
        player: Player,
        rigid_body: RigidBody::Dynamic,
        collider: Collider::capsule_endpoints(Vec2::new(-5.0, 0.0), Vec2::new(10.0, 0.0), 21.0),
        external_force: ExternalForce::ZERO,
        locked_axes: LockedAxes::new().lock_rotation(),
        gravity_scale: GravityScale(1.0),
        collision_layer: CollisionLayers::new(
            [Layer::Player],
            [Layer::Rock, Layer::Wall, Layer::Paddle, Layer::Ball],
        ),
        restitution: Restitution::PERFECTLY_INELASTIC,
    };
    commands.spawn(pb);

    commands.spawn(BallBundle::new(&assets, &mut rng, PADDLE_START));

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
            commands.spawn(RockBundle::new(&assets, *image_index, rock_position));
        }
    }
}

#[derive(Component, Default)]
struct Player;

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    #[bundle()]
    input_manager: InputManagerBundle<Action>,
    #[bundle()]
    sprite: SpriteSheetBundle,
    animation_indices: AnimationIndices,
    collider: Collider,
    rigid_body: RigidBody,
    external_force: ExternalForce,
    locked_axes: LockedAxes,
    gravity_scale: GravityScale,
    collision_layer: CollisionLayers,
    restitution: Restitution,
}

#[derive(Component)]
struct Paddle {
    left: bool,
}

#[derive(Bundle)]
struct PaddleBundle {
    paddle: Paddle,
    #[bundle()]
    sprite: SpriteBundle,
    collider: Collider,
    rigid_body: RigidBody,
    restitution: Restitution,
}

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
            collider: Collider::capsule_endpoints(
                Vec2::new(-20.0, 0.0),
                Vec2::new(20.0, 0.0),
                if image_index == 1 { 13.0 } else { 15.0 },
            ),
            // collider: Collider::cuboid(60.0, if image_index == 1 { 18.0 } else { 25.0 }),
            rigid_body: RigidBody::Static,
            collision_layer: CollisionLayers::new(
                [Layer::Rock],
                [Layer::Ball, Layer::Player, Layer::Wall],
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
}

impl BallBundle {
    fn new(assets: &Res<LDAssets>, rng: &mut Randomizer, paddle_location: Vec3) -> BallBundle {
        // Randomize starting direction of ball
        let angle = rng.rng.gen_range(-PI/4.0 .. PI / 4.0);
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
                [Layer::Rock, Layer::Player, Layer::Paddle, Layer::Wall],
            ),
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

fn spawn_ball(
    time: Res<Time>,
    assets: Res<LDAssets>,
    mut rng: ResMut<Randomizer>,
    mut ball_timer: ResMut<BallSpawnTimer>,
    mut commands: Commands,
    paddle: Query<&Transform, With<Paddle>>,
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

#[derive(Bundle)]
struct WallBundle {
    sprite_bundle: SpriteBundle,
    collider: Collider,
    rigid_body: RigidBody,
    restitution: Restitution,
    collision_layer: CollisionLayers,
}

impl WallBundle {
    // This "builder method" allows us to reuse logic across our wall entities,
    // making our code easier to read and less prone to bugs when we change the logic
    fn new(location: WallLocation) -> WallBundle {
        WallBundle {
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

fn player_input_map() -> InputMap<Action> {
    let mut input_map = InputMap::default();
    input_map.insert(
        UserInput::VirtualAxis(VirtualAxis {
            negative: KeyCode::Left.into(),
            positive: KeyCode::Right.into(),
        }),
        Action::Move,
    );
    input_map.insert(
        UserInput::VirtualAxis(VirtualAxis {
            negative: GamepadButtonType::DPadLeft.into(),
            positive: GamepadButtonType::DPadRight.into(),
        }),
        Action::Move,
    );
    input_map.insert(KeyCode::Up, Action::Jump);
    input_map.insert(GamepadButtonType::South, Action::Jump);
    input_map
}

fn player_inputs(
    mut player_query: Query<(&mut LinearVelocity, &ActionState<Action>), With<Player>>,
) {
    let Ok((mut velocity, action_state)) = player_query.get_single_mut() else {
        return;
    };

    if action_state.pressed(Action::Move) {
        let x_amount = action_state.clamped_value(Action::Move);
        velocity.x = x_amount * PLAYER_X_SPEED;
    }

    if action_state.just_pressed(Action::Jump) {
        // THIS IS NOT THE CORRECT WAY TO DO IT, SOLEN FROM:
        // https://github.com/Jondolf/bevy_xpbd/blob/8b2ea8fd4754fb3ecd51f79fad282d22631d2c7f/crates/bevy_xpbd_2d/examples/one_way_platform_2d.rs#L152-L157
        if velocity.y.abs() < 0.5 {
            velocity.y = 400f32;
        }
    }
}

fn check_for_gg(
    player_xform: Query<&Transform, With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(player_xform) = player_xform.get_single() else {
        return;
    };

    if player_xform.translation.y < -380.0 {
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
        With<Player>,
    >,
    player_animations: Res<PlayerAnimationTable>,
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
    mut ev_mid: EventReader<Collision>,
    balls: Query<Entity, With<Ball>>,
    mut linear_velocity: Query<&mut LinearVelocity>,
    collisions: Query<(Entity, Option<&Rock>), With<Collider>>,
) {
    for e in &mut ev_mid {
        let maybe_ball = balls
            .get(e.0.entity1)
            .ok()
            .or_else(|| balls.get(e.0.entity2).ok());

        if let Some(mut ball_v) = maybe_ball.and_then(|b| linear_velocity.get_mut(b).ok()) {
            /*
            let m = e.0.manifolds.first().unwrap();
            let first = balls.contains(e.0.entity1);
            let normal_normal = if first { m.normal1 } else { -m.normal1 };
            ball_v.0 = ball_v.0 - (ball_v.0.dot(normal_normal) * normal_normal * 2.0);
            */
            if let Some((e, maybe_rock)) = collisions
                .get(e.0.entity1)
                .ok()
                .or_else(|| collisions.get(e.0.entity2).ok())
            {
                if maybe_rock.is_some() {
                    commands.entity(e).despawn_recursive();
                }
            }
        }
    }
}
