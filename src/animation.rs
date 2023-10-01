use bevy::prelude::*;

#[derive(Component, Clone, Default)]
pub(crate) struct AnimationIndices {
    pub(crate) first: usize,
    pub(crate) last: usize,
    pub(crate) timer: Timer,
}

#[derive(Event)]
pub(crate) struct AnimationLoopCompleted {
    entity: Entity,
}

pub(crate) fn animate(
    time: Res<Time>,
    mut animated_sprites: Query<(
        Entity,
        &mut AnimationIndices,
        &mut TextureAtlasSprite,
    )>,
    mut completion: EventWriter<AnimationLoopCompleted>,
) {
    for (entity, mut indices, mut sprite) in &mut animated_sprites {
        indices.timer.tick(time.delta());
        if indices.timer.finished() {
            let mut new_index = sprite.index + 1;
            if new_index > indices.last {
                if indices.timer.mode() == TimerMode::Repeating {
                    new_index = indices.first;
                }
                completion.send(AnimationLoopCompleted { entity, });
            }
            sprite.index = new_index;
        }
    }
}
