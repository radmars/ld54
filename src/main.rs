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

use animation::{animate, AnimationIndices, AnimationLoopCompleted};
use bevy::{prelude::*, window::WindowResolution};
use bevy_asset_loader::prelude::*;
use iyes_progress::prelude::*;
use leafwing_input_manager::{axislike::VirtualAxis, prelude::*};
use rand::prelude::*;

const PLAYER_X_SPEED: f32 = 400.0;

const PADDLE_SIZE: Vec3 = Vec3::new(100.0, 20.0, 0.0);

const LEFT_WALL: f32 = -400.0;
const RIGHT_WALL: f32 = 400.0;
const BOTTOM_WALL: f32 = -300.0;
const TOP_WALL: f32 = 300.0;

const ROCK_WIDTH: f32 = 64.0;
const ROCK_HEIGHT: f32 = 52.0;

const GAP_BETWEEN_PADDLE_AND_TOP: f32 = 60.0;

const GAP_BETWEEN_ROCKS: f32 = 6.0;
const GAP_BETWEEN_ROCKS_AND_BOTTOM: f32 = 30.0;
const GAP_BETWEEN_ROCKS_AND_SIDES: f32 = 30.0;
const GAP_BETWEEN_ROCKS_AND_PADDLE: f32 = 200.0;

mod animation;

#[derive(Resource)]
struct PlayerAnimationTable {
    idle: AnimationIndices,
    walk: AnimationIndices,
    jump: AnimationIndices,
}

#[derive(States, Default, Copy, Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    #[default]
    Loading,
    Setup,
    Splash,
    Playing,
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
                    resolution: WindowResolution::new(800.0, 600.0),
                    transparent: true,
                    ..Default::default()
                }),
                ..default()
            })
            // Fix sprite blur
            .set(ImagePlugin::default_nearest()),
        loading_plugin,
        InputManagerPlugin::<Action>::default(),
        //  PhysicsPlugins::default(),
    ))
    .add_loading_state(loading_state)
    .add_collection_to_loading_state::<_, LDAssets>(loading_game_state)
    .add_state::<GameState>()
    .add_event::<AnimationLoopCompleted>()
    .insert_resource(PlayerAnimationTable {
        idle: AnimationIndices {
            first: 0,
            last: 0,
            timer: Timer::from_seconds(1., TimerMode::Repeating),
        },
        walk: AnimationIndices {
            first: 1,
            last: 2,
            timer: Timer::from_seconds(0.03, TimerMode::Repeating),
        },
        jump: AnimationIndices {
            first: 3,
            last: 7,
            timer: Timer::from_seconds(0.3, TimerMode::Repeating),
        },
    })
    .insert_resource(Msaa::Off)
    .insert_resource(ClearColor(Color::hex("#000000").unwrap()))
    .insert_resource(Randomizer::default())
    .add_systems(Update, bevy::window::close_on_esc)
    .add_systems(Update, (wait_to_start).run_if(in_state(GameState::Splash)))
    .add_systems(OnEnter(GameState::Setup), setup)
    .add_systems(OnEnter(GameState::Splash), splash_setup)
    .add_systems(OnExit(GameState::Splash), splash_exit)
    .add_systems(OnEnter(GameState::Playing), playing_setup)
    .add_systems(
        Update,
        (player_inputs, animate).run_if(in_state(GameState::Playing)),
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

    #[asset(path = "gamebg.png")]
    gamebg: Handle<Image>,

    #[asset(path = "splash.png")]
    splash: Handle<Image>,
}

fn setup(mut commands: Commands, mut next_state: ResMut<NextState<GameState>>) {
    commands.spawn(Camera2dBundle::default());
    next_state.set(GameState::Splash);
}

fn splash_setup(assets: Res<LDAssets>, mut commands: Commands) {
    commands.spawn(SpriteBundle {
        texture: assets.splash.clone(),
        ..default()
    });
}

fn splash_exit(mut commands: Commands, things_to_remove: Query<Entity, With<Sprite>>) {
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

    let idle_player = player_animations.idle.clone();

    let pb = PlayerBundle {
        sprite: SpriteSheetBundle {
            texture_atlas: assets.player.clone(),
            sprite: TextureAtlasSprite {
                index: idle_player.first,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(1.0, 2.0, 1.0)),
            ..default()
        },
        input_manager: InputManagerBundle::<Action> {
            input_map: player_input_map(),
            ..default()
        },
        animation_indices: idle_player,
        player: Player::default(),
    };
    commands.spawn(pb);

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

            // brick
            commands.spawn(RockBundle {
                sprite: SpriteSheetBundle {
                    texture_atlas: assets.rocks.clone(),
                    sprite: TextureAtlasSprite {
                        index: *image_index,
                        ..default()
                    },
                    transform: Transform {
                        translation: rock_position.extend(0.0),
                        ..default()
                    },
                    ..default()
                },
                ..default()
            });
        }
    }
}

#[derive(Component, Default)]
struct Player {}

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    #[bundle()]
    input_manager: InputManagerBundle<Action>,
    #[bundle()]
    sprite: SpriteSheetBundle,
    animation_indices: AnimationIndices,
}

#[derive(Component, Default)]
struct Rock {}

#[derive(Bundle, Default)]
struct RockBundle {
    rock: Rock,
    #[bundle()]
    sprite: SpriteSheetBundle,
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
    input_map.insert(KeyCode::Up, Action::Jump);
    input_map
}

fn maybe_change_animation(target: &mut AnimationIndices, source: &AnimationIndices) {
    if target.first != source.first {
        *target = source.clone();
    }
}

fn player_inputs(
    mut player_query: Query<
        (
            &mut Transform,
            &mut TextureAtlasSprite,
            &mut AnimationIndices,
            &ActionState<Action>,
        ),
        With<Player>,
    >,
    player_animations: Res<PlayerAnimationTable>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut atlas, mut animation, action_state)) = player_query.get_single_mut()
    else {
        return;
    };

    if action_state.pressed(Action::Move) {
        let x_amount = action_state.clamped_value(Action::Move);


        // TODO: Probably need to track jumping state or vertical velocity or
        // make animations nto input based.
        atlas.flip_x = x_amount < 0.0;
        maybe_change_animation(&mut animation, &player_animations.walk);

        transform.translation +=
            Vec3::new(PLAYER_X_SPEED * x_amount * time.delta_seconds(), 0.0, 0.0);
    }
    //  else {
    //    maybe_change_animation(&mut animation, &player_animations.idle);
    //}

    if action_state.just_pressed(Action::Jump) {
        maybe_change_animation(&mut animation, &player_animations.jump);
        transform.translation += Vec3::new(0.0, 20. * time.delta_seconds(), 0.0);
    }
}
