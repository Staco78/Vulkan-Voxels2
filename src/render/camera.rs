use std::time::Duration;

use glm::Vec3;
use nalgebra_glm as glm;
use nalgebra_glm::Mat4;
use vulkanalia::vk;

use crate::inputs::Inputs;

const SENSITIVITY: f32 = 3.;
const SPEED: f32 = 10.;
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
    pos: Vec3,
    yaw: f32,
    pitch: f32,

    proj: Mat4,
}

impl Camera {
    pub fn new(swapchain_extent: vk::Extent2D) -> Self {
        Self {
            pos: Vec3::new(2., 2., 2.),
            yaw: 230.,
            pitch: -35.,
            proj: Self::create_proj(swapchain_extent),
        }
    }

    pub fn tick(&mut self, inputs: &Inputs, elapsed: Duration) {
        let mouse_delta = inputs.fetch_mouse_delta();

        self.yaw += mouse_delta.0 as f32 * elapsed.as_secs_f32() * SENSITIVITY;
        self.pitch -= mouse_delta.1 as f32 * elapsed.as_secs_f32() * SENSITIVITY;

        if self.pitch > 89.0 {
            self.pitch = 89.0;
        }
        if self.pitch < -89.0 {
            self.pitch = -89.0;
        }

        let dir =
            Vec3::new(self.yaw.to_radians().cos(), 0., self.yaw.to_radians().sin()).normalize();
        let right = dir.cross(&Vec3::y()).normalize();
        let up = Vec3::y();

        let speed = SPEED * elapsed.as_secs_f32();

        if inputs.is_key_pressed(winit::event::VirtualKeyCode::Z) {
            self.pos += dir * speed;
        }
        if inputs.is_key_pressed(winit::event::VirtualKeyCode::S) {
            self.pos -= dir * speed;
        }
        if inputs.is_key_pressed(winit::event::VirtualKeyCode::Q) {
            self.pos -= right * speed;
        }
        if inputs.is_key_pressed(winit::event::VirtualKeyCode::D) {
            self.pos += right * speed;
        }
        if inputs.is_key_pressed(winit::event::VirtualKeyCode::Space) {
            self.pos += up * speed;
        }
        if inputs.is_key_pressed(winit::event::VirtualKeyCode::LShift) {
            self.pos -= up * speed;
        }
    }

    pub fn ubo(&self) -> UniformBufferObject {
        let mut front = Vec3::default();
        front.x = self.yaw.to_radians().cos() * self.pitch.to_radians().cos();
        front.y = self.pitch.to_radians().sin();
        front.z = self.yaw.to_radians().sin() * self.pitch.to_radians().cos();
        let rotation = front.normalize();
        let view = glm::look_at(&self.pos, &(self.pos + rotation), &glm::vec3(0.0, 1.0, 0.0));

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
