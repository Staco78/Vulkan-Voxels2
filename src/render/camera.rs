use std::time::Instant;

use nalgebra_glm as glm;
use nalgebra_glm::Mat4;
use vulkanalia::vk;

#[derive(Debug)]
#[repr(C)]
pub struct UniformBufferObject {
    model: Mat4,
    view: Mat4,
    proj: Mat4,
}

#[derive(Debug)]
pub struct Camera {
    start: Instant,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
    pub fn ubo(&self, swapchain_extent: vk::Extent2D) -> UniformBufferObject {
        let time = self.start.elapsed().as_secs_f32();
        let model = glm::rotate(
            &glm::identity(),
            time * glm::radians(&glm::vec1(90.0))[0],
            &glm::vec3(0.0, 0.0, 1.0),
        );
        let view = glm::look_at(
            &glm::vec3(2.0, 2.0, 2.0),
            &glm::vec3(0.0, 0.0, 0.0),
            &glm::vec3(0.0, 0.0, 1.0),
        );
        let mut proj = glm::perspective_rh_zo(
            swapchain_extent.width as f32 / swapchain_extent.height as f32,
            glm::radians(&glm::vec1(45.0))[0],
            0.1,
            10.0,
        );
        proj[(1, 1)] *= -1.0;
        UniformBufferObject { model, view, proj }
    }
}
