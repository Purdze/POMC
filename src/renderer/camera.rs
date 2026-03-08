use glam::{Mat4, Vec3};

use crate::window::input::InputState;

const UP: Vec3 = Vec3::Y;
const DEFAULT_FOV: f32 = 1.2217;
const NEAR: f32 = 0.1;
const FAR: f32 = 1000.0;
const SENSITIVITY: f32 = 0.003;
const PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.01;

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    aspect_ratio: f32,
}

impl Camera {
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 2.0, 5.0),
            yaw: 0.0,
            pitch: 0.0,
            aspect_ratio,
        }
    }

    pub fn update_look(&mut self, input: &mut InputState) {
        if input.is_cursor_captured() {
            let (dx, dy) = input.consume_mouse_delta();
            self.yaw -= dx as f32 * SENSITIVITY;
            self.pitch = (self.pitch - dy as f32 * SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        }
    }

    pub fn set_aspect_ratio(&mut self, aspect: f32) {
        self.aspect_ratio = aspect;
    }

    pub fn set_position(&mut self, position: Vec3, yaw_degrees: f32, pitch_degrees: f32) {
        self.position = position;
        self.yaw = yaw_degrees.to_radians();
        self.pitch = pitch_degrees.to_radians();
    }

    pub fn view_projection(&self) -> Mat4 {
        let forward = Vec3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        );
        let view = Mat4::look_to_rh(self.position, forward, UP);
        let proj = Mat4::perspective_rh(DEFAULT_FOV, self.aspect_ratio, NEAR, FAR);
        proj * view
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn from_camera(camera: &Camera) -> Self {
        Self {
            view_proj: camera.view_projection().to_cols_array_2d(),
        }
    }
}
