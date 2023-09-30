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

use bevy::{prelude::*, window::WindowResolution};
use bevy_asset_loader::prelude::*;
use iyes_progress::prelude::*;

#[derive(States, Default, Copy, Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    #[default]
    Loading,
    Setup,
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
        //  InputManagerPlugin::<Action>::default(),
        //  PhysicsPlugins::default(),
    ))
    .add_loading_state(loading_state)
    .add_collection_to_loading_state::<_, LDAssets>(loading_game_state)
    .add_state::<GameState>()
    .insert_resource(Msaa::Off)
    .insert_resource(ClearColor(Color::hex("#000000").unwrap()))
    .add_systems(Update, bevy::window::close_on_esc)
    .add_systems(Update, something)
    .run();
}

fn something(k: Res<Input<KeyCode>>) {
    if k.just_pressed(KeyCode::J) {
        info!("Hohoho");
    }
}

#[derive(AssetCollection, Resource)]
struct LDAssets {
    #[asset(texture_atlas(tile_size_x = 18., tile_size_y = 18., columns = 6, rows = 1))]
    #[asset(path = "player.png")]
    player: Handle<TextureAtlas>,

    #[asset(path = "gamebg.png")]
    gamebg: Handle<Image>,

    #[asset(path = "splash.png")]
    splash: Handle<Image>,
}
