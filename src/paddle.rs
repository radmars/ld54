use bevy::prelude::*;

use crate::{LDAssets, PADDLE_START};
use bevy_xpbd_2d::prelude::*;

#[derive(Component)]
pub(crate) struct Paddle {
    pub(crate) left: bool,
}

#[derive(Bundle)]
pub(crate) struct PaddleBundle {
    paddle: Paddle,
    #[bundle()]
    sprite: SpriteBundle,
    collider: Collider,
    rigid_body: RigidBody,
    restitution: Restitution,
}

impl PaddleBundle {
    pub(crate) fn new(assets: &LDAssets) -> Self {
        PaddleBundle {
            paddle: Paddle { left: true },
            sprite: SpriteBundle {
                texture: assets.paddle.clone(),
                transform: Transform::from_translation(PADDLE_START),
                ..Default::default()
            },
            collider: Collider::capsule_endpoints(
                Vec2::new(-11.0, -8.0),
                Vec2::new(11.0, -8.0),
                15.0,
            ),
            rigid_body: RigidBody::Static,
            restitution: Restitution::new(1.0).with_combine_rule(CoefficientCombine::Max),
        }
    }
}
