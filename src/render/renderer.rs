use std::{
    fmt::Debug,
    mem::size_of,
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY},
    vk::{self, DeviceV1_0, Handle, HasBuilder, KhrSwapchainExtension},
    Entry,
};
use winit::window::Window;

use crate::{
    inputs::Inputs,
    options::AppOptions,
    render::{camera::UniformBufferObject, devices::Device, uniform::Uniforms},
    shader_module,
    world::{chunks::Chunks, ChunkPos, EntityPos},
};

use super::{
    camera::Camera,
    commands::{CommandBuffer, CommandPool},
    depth::DepthBuffer,
    descriptors::DescriptorSetLayout,
    devices::{self, DEVICE},
    framebuffers::Framebuffers,
    gui_renderer::GuiRenderer,
    instance::Instance,
    memory::init_allocator,
    pipeline::{Pipeline, PipelineCreationOptions},
    queues::QUEUES,
    render_pass::{RenderPass, RenderPassCreationOptions},
    surface::Surface,
    swapchain::Swapchain,
    sync::{Fences, Semaphores},
    vertex::Vertex,
    RegionsManager,
};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[derive(Debug)]
pub struct Renderer {
    gui_renderer: GuiRenderer,

    images_in_flight: Fences,
    in_flight_fences: Fences,
    render_finished_semaphores: Semaphores,
    image_available_semaphores: Semaphores,
    command_buffers: Vec<CommandBuffer>,
    command_pool: CommandPool,
    framebuffers: Framebuffers,
    depth_buffer: DepthBuffer,
    pipeline: Pipeline,
    render_pass: RenderPass,
    uniforms: Uniforms<UniformBufferObject>,
    swapchain: Swapchain,
    physical_device: vk::PhysicalDevice,
    surface: Surface,
    _entry: Entry,

    frame: usize,
    camera: Camera,
    pub regions: Arc<RegionsManager>,
}

impl Renderer {
    pub fn new(window: &Window, chunks: Arc<RwLock<Chunks>>) -> Result<Self> {
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
        let render_pass_options =
            RenderPassCreationOptions::default(&swapchain).with_depth(physical_device)?;
        let render_pass =
            RenderPass::new(&render_pass_options).context("Render pass creation failed")?;
        let pipeline_options = Self::create_pipeline_options(&uniforms.layout)
            .context("Pipeline options creation failed")?;
        let pipeline = Pipeline::new::<Vertex>(&swapchain, &render_pass, &pipeline_options)
            .context("Pipeline creation failed")?;
        let depth_buffer = DepthBuffer::new(physical_device, &swapchain)
            .context("Depth buffer creation failed")?;
        let framebuffers = Framebuffers::new(&swapchain, &render_pass, &depth_buffer)?;
        let mut command_pool = CommandPool::new(QUEUES.get_default_graphics().family)?;
        let command_buffers = command_pool
            .alloc_buffers(framebuffers.count(), false)
            .context("Command buffers allocation failed")?;
        let gui_renderer = GuiRenderer::new(&swapchain, &render_pass, &mut command_pool)
            .context("Gui renderer creation failed")?;
        let render_finished_semaphores = Semaphores::new(MAX_FRAMES_IN_FLIGHT)?;
        let image_available_semaphores = Semaphores::new(MAX_FRAMES_IN_FLIGHT)?;
        let in_flight_fences = Fences::new(MAX_FRAMES_IN_FLIGHT, true)?;
        let images_in_flight = Fences::from_vec(vec![vk::Fence::null(); swapchain.images.len()]);

        let camera = Camera::new(swapchain.extent);

        let regions = Arc::new(
            RegionsManager::new(chunks, swapchain.images.len())
                .context("Region manager creation failed")?,
        );

        Ok(Self {
            _entry: entry,
            surface,
            physical_device,
            swapchain,
            uniforms,
            render_pass,
            pipeline,
            depth_buffer,
            framebuffers,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,

            gui_renderer,

            frame: 0,
            camera,
            regions,
        })
    }

    fn create_pipeline_options(layout: &DescriptorSetLayout) -> Result<PipelineCreationOptions> {
        let push_constant_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(size_of::<ChunkPos>() as u32)
            .build();
        Ok(PipelineCreationOptions {
            shaders: vec![
                (shader_module!("shader.vert")?, vk::ShaderStageFlags::VERTEX),
                (
                    shader_module!("shader.frag")?,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
            ],
            cull_mode: vk::CullModeFlags::BACK,
            polygon_mode: AppOptions::get().polygon_mode,
            descriptors_layouts: vec![layout],
            push_constant_ranges: vec![push_constant_range],
            blend_attachment: vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::all())
                .build(),
            dynamic_state: Default::default(),
        })
    }

    pub fn render(
        &mut self,
        elapsed: Duration,
        window: &Window,
        inputs: &Inputs,
        gui_primitives: &[egui::ClippedPrimitive],
        gui_textures_delta: egui::TexturesDelta,
    ) -> Result<()> {
        self.camera.tick(inputs, elapsed);

        unsafe { DEVICE.wait_for_fences(&[self.in_flight_fences[self.frame]], true, u64::MAX) }
            .context("Fence waiting failed")?;

        let result = unsafe {
            DEVICE.acquire_next_image_khr(
                self.swapchain.swapchain,
                u64::MAX,
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
                    u64::MAX,
                )
            }
            .context("Fence waiting failed")?;
        }

        // Commands recording
        let command_buff = &mut self.command_buffers[image_index as usize];
        {
            command_buff.reset()?;
            command_buff.begin()?;
            let render_area = vk::Rect2D::builder()
                .offset(vk::Offset2D::default())
                .extent(self.swapchain.extent);
            let color_clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
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
                .render_pass(*self.render_pass)
                .framebuffer(self.framebuffers[image_index as usize])
                .render_area(render_area)
                .clear_values(clear_values);
            unsafe {
                DEVICE.cmd_begin_render_pass(
                    **command_buff,
                    &info,
                    vk::SubpassContents::SECONDARY_COMMAND_BUFFERS,
                );
            }

            let inheritance_info = vk::CommandBufferInheritanceInfo::builder()
                .render_pass(*self.render_pass)
                .subpass(0)
                .framebuffer(self.framebuffers[image_index as usize]);

            for region in self.regions.inner().values_mut() {
                unsafe {
                    DEVICE.cmd_execute_commands(
                        **command_buff,
                        &[region
                            .fetch_cmd_buff(
                                image_index as usize,
                                &self.pipeline,
                                *self.uniforms[image_index as usize].descriptor_set,
                                &inheritance_info,
                            )
                            .context("Secondary cmd buff recording failed")?],
                    )
                }
            }

            let gui_buff = self
                .gui_renderer
                .render(
                    image_index as usize,
                    gui_primitives,
                    gui_textures_delta,
                    &inheritance_info,
                )
                .context("Gui rendering failed")?;

            unsafe {
                DEVICE.cmd_execute_commands(**command_buff, &[gui_buff]);
                DEVICE.cmd_end_render_pass(**command_buff);
            };

            command_buff.end()?;
        }

        self.images_in_flight[image_index as usize] = self.in_flight_fences[self.frame];

        self.uniforms[image_index as usize].write(self.camera.ubo());

        let wait_semaphores = &[self.image_available_semaphores[self.frame]];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = &[**command_buff];
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
                    *DEVICE.graphics_queue,
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

        let result = unsafe { DEVICE.queue_present_khr(*DEVICE.graphics_queue, &present_info) };
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
        unsafe { DEVICE.queue_wait_idle(*DEVICE.graphics_queue) }
            .context("Graphics queue wait idle failed")?;
        self.swapchain
            .recreate(self.physical_device, window, *self.surface)
            .context("New swapchain creation failed")?;
        self.depth_buffer
            .recreate(self.physical_device, &self.swapchain)
            .context("Depth buffer recreation failed")?;
        self.recreate_pipeline()?;
        self.command_pool
            .realloc_buffers(&mut self.command_buffers, self.framebuffers.count(), false)
            .context("Command buffers reallocation failed")?;
        self.images_in_flight
            .resize(self.swapchain.images.len(), vk::Fence::null());
        self.camera.rebuild_proj(self.swapchain.extent);

        Ok(())
    }

    pub fn recreate_pipeline(&mut self) -> Result<()> {
        unsafe { DEVICE.queue_wait_idle(*DEVICE.graphics_queue) }
            .context("Graphics queue wait idle failed")?;
        let render_pass_options =
            RenderPassCreationOptions::default(&self.swapchain).with_depth(self.physical_device)?;
        self.render_pass
            .recreate(&render_pass_options)
            .context("Render pass recreation failed")?;
        let pipeline_options = Self::create_pipeline_options(&self.uniforms.layout)
            .context("Pipeline options creation failed")?;
        self.pipeline
            .recreate::<Vertex>(&self.swapchain, &self.render_pass, &pipeline_options)
            .context("Pipeline recreation failed")?;
        self.framebuffers
            .recreate(&self.swapchain, &self.render_pass, &self.depth_buffer)
            .context("Framebuffers recreation failed")?;
        self.gui_renderer
            .recreate(&self.swapchain, &self.render_pass)?;
        self.regions
            .pipeline_recreated(self.swapchain.images.len())
            .context("Regions pipeline recreation handling failed")?;
        Ok(())
    }

    #[inline]
    pub fn camera_pos(&self) -> EntityPos {
        self.camera.pos
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let _ = DEVICE.queue_wait_idle(*DEVICE.graphics_queue);
        }
        // Prevent destructor to destroy null or already destroyed fences.
        self.images_in_flight.clear();
    }
}
