use bevy::{prelude::*, sprite::Anchor};
use bevy_xpbd_2d::prelude::*;
use leafwing_input_manager::{axislike::VirtualAxis, prelude::*};

use crate::{animation::AnimationIndices, Action, LDAssets, Layer};

#[derive(Resource)]
pub(crate) struct PlayerAnimationTable {
    pub(crate) idle: AnimationIndices,
    pub(crate) walk: AnimationIndices,
    pub(crate) jump_up: AnimationIndices,
    pub(crate) jump_down: AnimationIndices,
}

impl Default for PlayerAnimationTable {
    fn default() -> Self {
        PlayerAnimationTable {
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
        }
    }
}

#[derive(Component)]
pub(crate) struct PlayerSensor {}

#[derive(Bundle)]
pub(crate) struct PlayerSensorBundle {
    sprite: SpriteBundle,
    player_sensor: PlayerSensor,
    sensor: Sensor,
    collision_layer: CollisionLayers,
    rigid_body: RigidBody,
    collider: Collider,
}

#[derive(Component, Default)]
pub(crate) struct Player;

#[derive(Bundle)]
pub(crate) struct PlayerBundle {
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
    sleeping_disabled: SleepingDisabled,
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

impl PlayerBundle {
    pub(crate) fn new(assets: &LDAssets, animations: &PlayerAnimationTable) -> Self {
        let idle_player = animations.idle.clone();

        PlayerBundle {
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
            restitution: Restitution::PERFECTLY_INELASTIC
                .with_combine_rule(CoefficientCombine::Min),
            sleeping_disabled: SleepingDisabled,
        }
    }
}
