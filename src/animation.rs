use bevy::prelude::*;

#[derive(Component, Clone, Default)]
pub(crate) struct AnimationIndices {
    pub(crate) first: usize,
    pub(crate) last: usize,
    pub(crate) timer: Timer,
}

pub(crate) fn animate(
    time: Res<Time>,
    mut animated_sprites: Query<(&mut AnimationIndices, &mut TextureAtlasSprite)>,
) {
    for (mut indices, mut sprite) in &mut animated_sprites {
        indices.timer.tick(time.delta());
        if indices.timer.finished() {
            let new_index = sprite.index + 1;
            if new_index > indices.last {
                if indices.timer.mode() == TimerMode::Repeating {
                    sprite.index = indices.first;
                }
            } else {
                sprite.index = new_index;
            }
        }
    }
}
