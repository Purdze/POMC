use glam::Vec3;
use winit::keyboard::KeyCode;

use super::aabb::Aabb;
use super::collision::resolve_collision;
use crate::player::LocalPlayer;
use crate::window::input::InputState;
use crate::world::chunk::ChunkStore;

const GRAVITY: f32 = 0.08;
const JUMP_VELOCITY: f32 = 0.42;
const DRAG_AIR: f32 = 0.98;
const FRICTION_GROUND: f32 = 0.91;
const GROUND_ACCELERATION: f32 = 0.1;
const AIR_ACCELERATION: f32 = 0.02;
const PLAYER_HALF_WIDTH: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.8;

pub fn tick(player: &mut LocalPlayer, input: &InputState, chunk_store: &ChunkStore) {
    let (forward, strafe) = movement_input(input, player.yaw);

    let accel = if player.on_ground {
        GROUND_ACCELERATION
    } else {
        AIR_ACCELERATION
    };

    player.velocity.x += (forward.x + strafe.x) * accel;
    player.velocity.z += (forward.z + strafe.z) * accel;

    if player.on_ground && input.key_pressed(KeyCode::Space) {
        player.velocity.y = JUMP_VELOCITY;
    }

    player.velocity.y -= GRAVITY;
    player.velocity.y *= DRAG_AIR;

    let friction = if player.on_ground {
        FRICTION_GROUND
    } else {
        1.0
    };
    player.velocity.x *= friction;
    player.velocity.z *= friction;

    let aabb = Aabb::from_center(player.position, PLAYER_HALF_WIDTH, PLAYER_HEIGHT / 2.0);
    let (resolved, on_ground) = resolve_collision(chunk_store, aabb, player.velocity);

    player.position += resolved;
    player.on_ground = on_ground;

    if on_ground {
        player.velocity.y = 0.0;
    }
}

fn movement_input(input: &InputState, yaw: f32) -> (Vec3, Vec3) {
    let forward_dir = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right_dir = Vec3::new(-forward_dir.z, 0.0, forward_dir.x);

    let mut forward = Vec3::ZERO;
    let mut strafe = Vec3::ZERO;

    if input.key_pressed(KeyCode::KeyW) {
        forward += forward_dir;
    }
    if input.key_pressed(KeyCode::KeyS) {
        forward -= forward_dir;
    }
    if input.key_pressed(KeyCode::KeyA) {
        strafe -= right_dir;
    }
    if input.key_pressed(KeyCode::KeyD) {
        strafe += right_dir;
    }

    (forward, strafe)
}
