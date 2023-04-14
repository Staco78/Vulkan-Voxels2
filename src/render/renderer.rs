use std::{fmt::Debug, mem::size_of_val};

use anyhow::{anyhow, Context, Result};
use nalgebra_glm as glm;
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY},
    vk::{self, DeviceV1_0, Handle, HasBuilder, KhrSwapchainExtension},
    Entry,
};
use winit::window::Window;

use crate::render::{camera::UniformBufferObject, devices::Device, uniform::Uniforms};

use super::{
    buffer::Buffer,
    camera::Camera,
    commands::{CommandBuffer, CommandPool},
    depth::DepthBuffer,
    devices::{self, DEVICE},
    framebuffers::Framebuffers,
    instance::Instance,
    memory::init_allocator,
    pipeline::Pipeline,
    surface::Surface,
    swapchain::Swapchain,
    sync::{Fences, Semaphores},
    vertex::Vertex,
};

const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[derive(Debug)]
pub struct Renderer {
    vertex_buffer: Buffer,
    images_in_flight: Fences,
    in_flight_fences: Fences,
    render_finished_semaphores: Semaphores,
    image_available_semaphores: Semaphores,
    command_buffers: Vec<CommandBuffer>,
    command_pool: CommandPool,
    framebuffers: Framebuffers,
    depth_buffer: DepthBuffer,
    pipeline: Pipeline,
    uniforms: Uniforms<UniformBufferObject>,
    swapchain: Swapchain,
    physical_device: vk::PhysicalDevice,
    surface: Surface,
    _entry: Entry,

    frame: usize,
    camera: Camera,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        let loader = unsafe { LibloadingLoader::new(LIBRARY) }
            .with_context(|| format!("{} not found", LIBRARY))?;
        let entry = unsafe { Entry::new(loader) }.expect("Entry creation");
        Instance::init(&entry, window).context("Instance creation failed")?;
        let surface = Surface::new(window)?;
        let physical_device =
            devices::pick_physical(*surface).context("Physical device selection failed")?;
        Device::init(physical_device, *surface).context("Device creation failed")?;
        init_allocator(physical_device);
        let swapchain = Swapchain::new(physical_device, window, *surface)
            .context("Swapchain creation failed")?;
        let uniforms = Uniforms::<UniformBufferObject>::new(swapchain.images.len())
            .context("Uniforms creation failed")?;
        let pipeline = Pipeline::new(physical_device, &swapchain, &uniforms)
            .context("Pipeline creation failed")?;
        let depth_buffer = DepthBuffer::new(physical_device, &swapchain)
            .context("Depth buffer creation failed")?;
        let framebuffers = Framebuffers::new(&swapchain, &pipeline, &depth_buffer)?;
        let command_pool = CommandPool::new(physical_device)?;
        let command_buffers = command_pool
            .alloc_buffers(framebuffers.count())
            .context("Command buffers allocation failed")?;
        let render_finished_semaphores = Semaphores::new(MAX_FRAMES_IN_FLIGHT)?;
        let image_available_semaphores = Semaphores::new(MAX_FRAMES_IN_FLIGHT)?;
        let in_flight_fences = Fences::new(MAX_FRAMES_IN_FLIGHT, true)?;
        let images_in_flight = Fences::from_vec(vec![vk::Fence::null(); swapchain.images.len()]);
        let vertices = [
            Vertex::new(glm::vec3(-0.5, -0.5, 0.0), glm::vec3(1.0, 0.0, 0.0)),
            Vertex::new(glm::vec3(0.5, -0.5, 0.0), glm::vec3(0.0, 1.0, 0.0)),
            Vertex::new(glm::vec3(0.5, 0.5, 0.0), glm::vec3(0.0, 0.0, 1.0)),
            Vertex::new(glm::vec3(0.5, 0.5, 0.0), glm::vec3(0.0, 0.0, 1.0)),
            Vertex::new(glm::vec3(-0.5, 0.5, 0.0), glm::vec3(1.0, 1.0, 1.0)),
            Vertex::new(glm::vec3(-0.5, -0.5, 0.0), glm::vec3(1.0, 0.0, 0.0)),
            Vertex::new(glm::vec3(-0.5, -0.5, -0.5), glm::vec3(1.0, 0.0, 0.0)),
            Vertex::new(glm::vec3(0.5, -0.5, -0.5), glm::vec3(0.0, 1.0, 0.0)),
            Vertex::new(glm::vec3(0.5, 0.5, -0.5), glm::vec3(0.0, 0.0, 1.0)),
            Vertex::new(glm::vec3(0.5, 0.5, -0.5), glm::vec3(0.0, 0.0, 1.0)),
            Vertex::new(glm::vec3(-0.5, 0.5, -0.5), glm::vec3(1.0, 1.0, 1.0)),
            Vertex::new(glm::vec3(-0.5, -0.5, -0.5), glm::vec3(1.0, 0.0, 0.0)),
        ];
        let mut vertex_buffer =
            Buffer::new(size_of_val(&vertices), vk::BufferUsageFlags::VERTEX_BUFFER)
                .context("Vertex buffer creation failed")?;
        vertex_buffer
            .fill(&vertices)
            .context("Vertex buffer filling failed")?;
        let camera = Camera::new();

        let mut s = Self {
            _entry: entry,
            surface,
            physical_device,
            swapchain,
            uniforms,
            pipeline,
            depth_buffer,
            framebuffers,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
            vertex_buffer,

            frame: 0,
            camera,
        };

        s.record_commands().context("Commands recording failed")?;

        Ok(s)
    }

    fn record_commands(&mut self) -> Result<()> {
        for i in 0..self.command_buffers.len() {
            let buffer = &mut self.command_buffers[i];
            buffer.begin().context("Command buffer begining failed")?;

            let render_area = vk::Rect2D::builder()
                .offset(vk::Offset2D::default())
                .extent(self.swapchain.extent);
            let color_clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            };
            let depth_clear_value = vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            };
            let clear_values = &[color_clear_value, depth_clear_value];
            let info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.pipeline.render_pass)
                .framebuffer(self.framebuffers[i])
                .render_area(render_area)
                .clear_values(clear_values);
            unsafe {
                DEVICE.cmd_begin_render_pass(buffer.buffer, &info, vk::SubpassContents::INLINE);
                DEVICE.cmd_bind_pipeline(
                    buffer.buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.pipeline,
                );
                DEVICE.cmd_bind_vertex_buffers(
                    buffer.buffer,
                    0,
                    &[self.vertex_buffer.buffer],
                    &[0],
                );
                DEVICE.cmd_bind_descriptor_sets(
                    buffer.buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.layout,
                    0,
                    &[self.uniforms[i].descriptor_set],
                    &[],
                );
                DEVICE.cmd_draw(buffer.buffer, 12, 1, 0, 0);
                DEVICE.cmd_end_render_pass(buffer.buffer);
            };

            buffer.end().context("Command buffer ending failed")?;
        }

        Ok(())
    }

    pub fn render(&mut self, window: &Window) -> Result<()> {
        unsafe {
            DEVICE.wait_for_fences(&[self.in_flight_fences[self.frame]], true, u64::max_value())
        }
        .context("Fence waiting failed")?;

        let result = unsafe {
            DEVICE.acquire_next_image_khr(
                self.swapchain.swapchain,
                u64::max_value(),
                self.image_available_semaphores[self.frame],
                vk::Fence::null(),
            )
        };

        let image_index = match result {
            Ok((image_index, _)) => image_index,
            Err(vk::ErrorCode::OUT_OF_DATE_KHR) => {
                return self
                    .recreate_swapchain(window)
                    .context("Swapchain recreation failed")
            }
            Err(e) => return Err(anyhow!(e).context("Next image acquiring failed")),
        };

        if !self.images_in_flight[image_index as usize].is_null() {
            unsafe {
                DEVICE.wait_for_fences(
                    &[self.images_in_flight[image_index as usize]],
                    true,
                    u64::max_value(),
                )
            }
            .context("Fence waiting failed")?;
        }

        self.images_in_flight[image_index as usize] = self.in_flight_fences[self.frame];

        self.uniforms[image_index as usize].write(self.camera.ubo(self.swapchain.extent));

        let wait_semaphores = &[self.image_available_semaphores[self.frame]];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = &[self.command_buffers[image_index as usize].buffer];
        let signal_semaphores = &[self.render_finished_semaphores[self.frame]];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        unsafe {
            DEVICE
                .reset_fences(&[self.in_flight_fences[self.frame]])
                .context("Fence reset failaed")?;

            DEVICE
                .queue_submit(
                    DEVICE.graphics_queue,
                    &[submit_info],
                    self.in_flight_fences[self.frame],
                )
                .context("Queue submiting failed")?;
        };

        let swapchains = &[self.swapchain.swapchain];
        let image_indices = &[image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);

        let result = unsafe { DEVICE.queue_present_khr(DEVICE.present_queue, &present_info) };
        let changed = result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
            || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);

        if changed {
            self.recreate_swapchain(window)?;
        } else if let Err(e) = result {
            return Err(anyhow!(e).context("Presenting failed"));
        }

        self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    pub fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
        unsafe { DEVICE.device_wait_idle() }.context("Device wait idle failed")?;
        self.swapchain
            .recreate(self.physical_device, window, *self.surface)
            .context("New swapchain creation failed")?;
        self.depth_buffer
            .recreate(self.physical_device, &self.swapchain)
            .context("Depth buffer recreation failed")?;
        self.pipeline
            .recreate(self.physical_device, &self.swapchain, &self.uniforms)
            .context("Pipeline recreation failed")?;
        self.framebuffers
            .recreate(&self.swapchain, &self.pipeline, &self.depth_buffer)
            .context("Framebuffers recreation failed")?;
        self.command_pool
            .realloc_buffers(&mut self.command_buffers, self.framebuffers.count())
            .context("Command buffers reallocation failed")?;
        self.images_in_flight
            .resize(self.swapchain.images.len(), vk::Fence::null());
        self.record_commands()
            .context("Commands recording failed")?;

        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let _ = DEVICE.device_wait_idle();
        }
        // Prevent destructor to destroy null or already destroyed fences.
        self.images_in_flight.clear();
    }
}
