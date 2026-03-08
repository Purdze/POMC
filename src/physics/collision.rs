use glam::Vec3;

use super::aabb::Aabb;
use crate::world::chunk::ChunkStore;

pub fn collect_block_aabbs(chunk_store: &ChunkStore, region: &Aabb) -> Vec<Aabb> {
    let mut aabbs = Vec::new();

    let min_x = region.min.x.floor() as i32;
    let min_y = region.min.y.floor() as i32;
    let min_z = region.min.z.floor() as i32;
    let max_x = region.max.x.ceil() as i32;
    let max_y = region.max.y.ceil() as i32;
    let max_z = region.max.z.ceil() as i32;

    for by in min_y..max_y {
        for bz in min_z..max_z {
            for bx in min_x..max_x {
                let state = chunk_store.get_block_state(bx, by, bz);
                if !state.is_air() {
                    aabbs.push(Aabb::new(
                        Vec3::new(bx as f32, by as f32, bz as f32),
                        Vec3::new((bx + 1) as f32, (by + 1) as f32, (bz + 1) as f32),
                    ));
                }
            }
        }
    }

    aabbs
}

pub fn resolve_collision(
    chunk_store: &ChunkStore,
    player_aabb: Aabb,
    mut velocity: Vec3,
) -> (Vec3, bool) {
    let expanded = player_aabb.expand(velocity);
    let block_aabbs = collect_block_aabbs(chunk_store, &expanded);

    let original_y = velocity.y;

    // Resolve Y first (gravity), then X, then Z - matching vanilla's approach for sorted axes
    // when Y movement is dominant (falling/jumping)
    for block in &block_aabbs {
        velocity.y = block.clip_y_collide(&player_aabb, velocity.y);
    }
    let mut resolved = player_aabb.offset(Vec3::new(0.0, velocity.y, 0.0));

    for block in &block_aabbs {
        velocity.x = block.clip_x_collide(&resolved, velocity.x);
    }
    resolved = resolved.offset(Vec3::new(velocity.x, 0.0, 0.0));

    for block in &block_aabbs {
        velocity.z = block.clip_z_collide(&resolved, velocity.z);
    }

    let on_ground = original_y < 0.0 && velocity.y != original_y;

    (velocity, on_ground)
}
