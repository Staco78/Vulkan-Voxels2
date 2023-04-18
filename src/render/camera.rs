use std::time::Duration;

use glm::{TVec3, Vec3};
use nalgebra_glm as glm;
use nalgebra_glm::Mat4;
use vulkanalia::vk;

use crate::inputs::Inputs;
use crate::world::EntityPos;

const SENSITIVITY: f32 = 0.05;
const SPEED: f32 = 30.;
const FOV: f32 = 45.;
const NEAR: f32 = 0.1;
const FAR: f32 = 1000.;

#[derive(Debug)]
#[repr(C)]
pub struct UniformBufferObject {
    view: Mat4,
    proj: Mat4,
}

#[derive(Debug)]
pub struct Camera {
    pub pos: EntityPos,
    proj: Mat4,
}

impl Camera {
    pub fn new(swapchain_extent: vk::Extent2D) -> Self {
        Self {
            pos: EntityPos::new(0., 0., 0., 0., 0.),
            proj: Self::create_proj(swapchain_extent),
        }
    }

    pub fn tick(&mut self, inputs: &Inputs, elapsed: Duration) {
        let mouse_delta = inputs.fetch_mouse_delta();

        let yaw = self.pos.yaw() + mouse_delta.0 as f32 * SENSITIVITY;
        let mut pitch = self.pos.pitch() - mouse_delta.1 as f32 * SENSITIVITY;

        if pitch > 89.0 {
            pitch = 89.0;
        }
        if pitch < -89.0 {
            pitch = -89.0;
        }

        let dir = Vec3::new(yaw.to_radians().cos(), 0., yaw.to_radians().sin()).normalize();
        let right = dir.cross(&Vec3::y()).normalize();
        let up = Vec3::y();

        let speed = SPEED * elapsed.as_secs_f32();

        let pos: &mut Vec3 = &mut self.pos;

        if inputs.is_key_pressed(winit::event::VirtualKeyCode::Z) {
            *pos += dir * speed;
        }
        if inputs.is_key_pressed(winit::event::VirtualKeyCode::S) {
            *pos -= dir * speed;
        }
        if inputs.is_key_pressed(winit::event::VirtualKeyCode::Q) {
            *pos -= right * speed;
        }
        if inputs.is_key_pressed(winit::event::VirtualKeyCode::D) {
            *pos += right * speed;
        }
        if inputs.is_key_pressed(winit::event::VirtualKeyCode::Space) {
            *pos += up * speed;
        }
        if inputs.is_key_pressed(winit::event::VirtualKeyCode::LShift) {
            *pos -= up * speed;
        }

        self.pos.look.x = pitch;
        self.pos.look.y = yaw;
    }

    pub fn ubo(&self) -> UniformBufferObject {
        let mut front = TVec3::default();
        front.x = self.pos.yaw().to_radians().cos() * self.pos.pitch().to_radians().cos();
        front.y = self.pos.pitch().to_radians().sin();
        front.z = self.pos.yaw().to_radians().sin() * self.pos.pitch().to_radians().cos();
        let rotation = front.normalize();
        let view = glm::look_at(
            &self.pos,
            &(*self.pos + rotation),
            &glm::vec3(0.0, 1.0, 0.0),
        );

        UniformBufferObject {
            view,
            proj: self.proj,
        }
    }

    #[inline]
    pub fn rebuild_proj(&mut self, swapchain_extent: vk::Extent2D) {
        self.proj = Self::create_proj(swapchain_extent);
    }

    fn create_proj(swapchain_extent: vk::Extent2D) -> Mat4 {
        let mut proj = glm::perspective_rh_zo(
            swapchain_extent.width as f32 / swapchain_extent.height as f32,
            FOV.to_radians(),
            NEAR,
            FAR,
        );
        proj[(1, 1)] *= -1.0;
        proj
    }
}
