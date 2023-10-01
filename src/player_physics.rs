use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
};

use crate::{
    animation::{maybe_change_animation, AnimationIndices},
    Player, PlayerAnimationTable, Rock, Size, Velocity,
};

#[derive(Default)]
struct PlayerCollision {
    left: bool,
    right: bool,
    down: bool,
}

pub(crate) fn player_physics(
    mut player_query: Query<
        (
            &mut Velocity,
            &mut Transform,
            &mut TextureAtlasSprite,
            &mut AnimationIndices,
            &Size,
        ),
        With<Player>,
    >,
    collider_query: Query<(&Transform, &Size), (With<Rock>, Without<Player>)>,
    player_animations: Res<PlayerAnimationTable>,
    time: Res<Time>,
) {
    let Ok((mut velocity, mut transform, mut atlas, mut animation, size)) =
        player_query.get_single_mut()
    else {
        return;
    };

    // Priority is dealing with jump, then dealing with walk.

    let delta = time.delta().as_secs_f32();

    // Check which directions are blockedk
    let pc = check_for_player_collisions(size, &transform, collider_query);
    info!("Player down: {}, left: {}, right: {}", pc.down, pc.left, pc.right);

    if pc.left && velocity.x < 0.0 {
        velocity.x = 0.0;
    }
    else{
        // THIS IS BAD PHYSICS: DELTA TIME IS NOT SUFFICIENT FOR THIS.
        velocity.y -= 2200.0 * delta;
    }

    if pc.right && velocity.x > 0.0 {
        velocity.x = 0.0;
    }
    if pc.down && velocity.y < 0.0 {
        velocity.y = 0.0;
        // TODO: Min uncollide
        //   transform.translation.y = 0.0;
    }

    if velocity.y.abs() > f32::EPSILON {
        if velocity.y < 0.0 {
            maybe_change_animation(&mut animation, &player_animations.jump_down);
        } else {
            maybe_change_animation(&mut animation, &player_animations.jump_up);
        }
        transform.translation.y += velocity.y * delta;
    } else if velocity.x.abs() > f32::EPSILON {
        maybe_change_animation(&mut animation, &player_animations.walk);
    } else {
        maybe_change_animation(&mut animation, &player_animations.idle);
    }

    if velocity.x.abs() > f32::EPSILON {
        transform.translation.x = (transform.translation.x + velocity.x * delta).clamp(-380.0, 380.0);
        atlas.flip_x = velocity.x < 0.0;
    }
}

fn check_for_player_collisions(
    player_size: &Size,
    player_transform: &Transform,
    collider_query: Query<(&Transform, &Size), (With<Rock>, Without<Player>)>,
) -> PlayerCollision {
    let mut player_collision = PlayerCollision::default();

    // check if the ball has collided with any other entity
    for (transform, size) in &collider_query {
        let collision = collide(
            player_transform.translation,
            player_size.0,
            transform.translation,
            size.0,
        );

        // TODO: Return overlap and negate?
        match collision {
            Some(Collision::Left) => player_collision.left = true,
            Some(Collision::Right) => player_collision.right = true,
            Some(Collision::Bottom) => player_collision.down = true,
            _ => {}
        };
    }

    player_collision
}
