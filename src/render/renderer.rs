use std::fmt::Debug;

use anyhow::{anyhow, Context, Result};
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY},
    vk::{
        self, DebugUtilsMessengerEXT, DeviceV1_0, ExtDebugUtilsExtension, Handle, HasBuilder,
        InstanceV1_0, KhrSurfaceExtension, KhrSwapchainExtension, SurfaceKHR,
    },
    window::create_surface,
    Entry, Instance,
};
use winit::window::Window;

use crate::render::{devices::create_device_and_queues, instance::create_instance};

use super::{
    commands::{CommandBuffer, CommandPool},
    devices::{self},
    framebuffers::Framebuffers,
    pipeline::Pipeline,
    swapchain::Swapchain,
    sync::{Fences, Semaphores},
};

const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[derive(Debug)]
pub struct Renderer {
    _entry: Entry,
    instance: Instance,
    debug_messenger: Option<DebugUtilsMessengerEXT>,
    surface: SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: vulkanalia::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain: Swapchain,
    pipeline: Pipeline,
    framebuffers: Framebuffers,
    command_pool: CommandPool,
    command_buffers: Vec<CommandBuffer>,
    image_available_semaphores: Semaphores,
    render_finished_semaphores: Semaphores,
    in_flight_fences: Fences,
    images_in_flight: Fences,

    frame: usize,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        let loader = unsafe { LibloadingLoader::new(LIBRARY) }
            .with_context(|| format!("{} not found", LIBRARY))?;
        let entry = unsafe { Entry::new(loader) }.expect("Entry creation");
        let (instance, debug_messenger) =
            create_instance(&entry, window).context("Instance creation failed")?;
        let surface = unsafe { create_surface(&instance, window, window) }
            .context("Surface creation failed")?;
        let physical_device = devices::pick_physical(&instance, surface)
            .context("Physical device selection failed")?;
        let (device, graphics_queue, present_queue) =
            create_device_and_queues(&instance, physical_device, surface)
                .context("Device creation failed")?;
        let swapchain = Swapchain::new(&instance, physical_device, &device, window, surface)
            .context("Swapchain creation failed")?;
        let pipeline = Pipeline::new(&device, &swapchain).context("Pipeline creation failed")?;
        let framebuffers = Framebuffers::new(&device, &swapchain, &pipeline)?;
        let command_pool = CommandPool::new(&instance, &device, physical_device)?;
        let command_buffers = command_pool
            .alloc_buffers(&device, framebuffers.count())
            .context("Command buffers allocation failed")?;
        let image_available_semaphores = Semaphores::new(&device, MAX_FRAMES_IN_FLIGHT)?;
        let render_finished_semaphores = Semaphores::new(&device, MAX_FRAMES_IN_FLIGHT)?;
        let in_flight_fences = Fences::new(&device, MAX_FRAMES_IN_FLIGHT, true)?;
        let images_in_flight = Fences::from_vec(vec![vk::Fence::null(); swapchain.images.len()]);

        let mut s = Self {
            _entry: entry,
            instance,
            debug_messenger,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain,
            pipeline,
            framebuffers,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,

            frame: 0,
        };

        s.record_commands().context("Commands recording failed")?;

        Ok(s)
    }

    fn record_commands(&mut self) -> Result<()> {
        for i in 0..self.command_buffers.len() {
            let buffer = &mut self.command_buffers[i];
            buffer
                .begin(&self.device)
                .context("Command buffer begining failed")?;

            let render_area = vk::Rect2D::builder()
                .offset(vk::Offset2D::default())
                .extent(self.swapchain.extent);
            let color_clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            };
            let clear_values = &[color_clear_value];
            let info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.pipeline.render_pass)
                .framebuffer(self.framebuffers[i])
                .render_area(render_area)
                .clear_values(clear_values);
            unsafe {
                self.device.cmd_begin_render_pass(
                    buffer.buffer,
                    &info,
                    vk::SubpassContents::INLINE,
                );
                self.device.cmd_bind_pipeline(
                    buffer.buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.pipeline,
                );
                self.device.cmd_draw(buffer.buffer, 3, 1, 0, 0);
                self.device.cmd_end_render_pass(buffer.buffer);
            };

            buffer
                .end(&self.device)
                .context("Command buffer ending failed")?;
        }

        Ok(())
    }

    pub fn render(&mut self, window: &Window) -> Result<()> {
        unsafe {
            self.device.wait_for_fences(
                &[self.in_flight_fences[self.frame]],
                true,
                u64::max_value(),
            )
        }
        .context("Fence waiting failed")?;

        let result = unsafe {
            self.device.acquire_next_image_khr(
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
                self.device.wait_for_fences(
                    &[self.images_in_flight[image_index as usize]],
                    true,
                    u64::max_value(),
                )
            }
            .context("Fence waiting failed")?;
        }

        self.images_in_flight[image_index as usize] = self.in_flight_fences[self.frame];

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
            self.device
                .reset_fences(&[self.in_flight_fences[self.frame]])
                .context("Fence reset failaed")?;

            self.device
                .queue_submit(
                    self.graphics_queue,
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

        let result = unsafe {
            self.device
                .queue_present_khr(self.present_queue, &present_info)
        };
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
        unsafe { self.device.device_wait_idle() }.context("Device wait idle failed")?;
        self.swapchain
            .recreate(
                &self.instance,
                self.physical_device,
                &self.device,
                window,
                self.surface,
            )
            .context("New swapchain creation failed")?;
        self.pipeline
            .recreate(&self.device, &self.swapchain)
            .context("Pipeline recreation failed")?;
        self.framebuffers
            .recreate(&self.device, &self.swapchain, &self.pipeline)
            .context("Framebuffers recreation failed")?;
        self.command_pool
            .realloc_buffers(
                &self.device,
                &mut self.command_buffers,
                self.framebuffers.count(),
            )
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
            let _ = self.device.device_wait_idle();
        }
        self.in_flight_fences.destroy(&self.device);
        self.render_finished_semaphores.destroy(&self.device);
        self.image_available_semaphores.destroy(&self.device);
        self.command_pool.destroy(&self.device);
        self.framebuffers.destroy(&self.device);
        self.pipeline.destroy(&self.device);
        self.swapchain.destroy(&self.device);
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_surface_khr(self.surface, None);
            if let Some(messenger) = self.debug_messenger {
                self.instance
                    .destroy_debug_utils_messenger_ext(messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}
