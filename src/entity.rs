use std::collections::HashMap;

use azalea_registry::builtin::EntityKind;
use glam::DVec3;

#[allow(dead_code)]
pub struct LivingEntity {
    pub position: DVec3,
    pub prev_position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub entity_type: EntityKind,
    pub walk_anim_pos: f32,
    pub walk_anim_speed: f32,
    pub is_baby: bool,
    pub on_ground: bool,
}

impl LivingEntity {
    pub fn new(entity_type: EntityKind, position: DVec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            prev_position: position,
            yaw,
            pitch,
            head_yaw: yaw,
            entity_type,
            walk_anim_pos: 0.0,
            walk_anim_speed: 0.0,
            is_baby: false,
            on_ground: false,
        }
    }
}

pub struct EntityStore {
    pub living: HashMap<i32, LivingEntity>,
}

impl EntityStore {
    pub fn new() -> Self {
        Self {
            living: HashMap::new(),
        }
    }

    pub fn spawn_living(
        &mut self,
        id: i32,
        entity_type: EntityKind,
        position: DVec3,
        yaw: f32,
        pitch: f32,
    ) {
        self.living
            .insert(id, LivingEntity::new(entity_type, position, yaw, pitch));
    }

    pub fn move_living_delta(&mut self, id: i32, dx: f64, dy: f64, dz: f64) {
        if let Some(entity) = self.living.get_mut(&id) {
            entity.prev_position = entity.position;
            entity.position += DVec3::new(dx, dy, dz);
        }
    }

    pub fn teleport_living(&mut self, id: i32, x: f64, y: f64, z: f64) {
        if let Some(entity) = self.living.get_mut(&id) {
            entity.prev_position = entity.position;
            entity.position = DVec3::new(x, y, z);
        }
    }

    pub fn set_baby(&mut self, id: i32, is_baby: bool) {
        if let Some(entity) = self.living.get_mut(&id) {
            entity.is_baby = is_baby;
        }
    }

    pub fn update_living_rotation(&mut self, id: i32, yaw: f32, pitch: f32) {
        if let Some(entity) = self.living.get_mut(&id) {
            entity.yaw = yaw;
            entity.pitch = pitch;
        }
    }

    pub fn remove_living(&mut self, id: i32) {
        self.living.remove(&id);
    }

    pub fn tick_living(&mut self) {
        for entity in self.living.values_mut() {
            let dx = entity.position.x - entity.prev_position.x;
            let dz = entity.position.z - entity.prev_position.z;
            let speed = ((dx * dx + dz * dz) as f32).sqrt();
            let target_speed = speed.min(1.0);
            entity.walk_anim_speed += (target_speed - entity.walk_anim_speed) * 0.4;
            entity.walk_anim_pos += entity.walk_anim_speed;
        }
    }

    pub fn clear(&mut self) {
        self.living.clear();
    }
}

pub fn is_living_mob(kind: &EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::Pig
            | EntityKind::Cow
            | EntityKind::Sheep
            | EntityKind::Chicken
            | EntityKind::Zombie
            | EntityKind::Skeleton
            | EntityKind::Creeper
            | EntityKind::Spider
    )
}
