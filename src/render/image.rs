use anyhow::{bail, Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use crate::render::memory::allocator;

use super::{devices::DEVICE, memory::Allocation, Buffer, CommandBuffer, Queue};

pub fn create_image_view(
    image: vk::Image,
    format: vk::Format,
    aspects: vk::ImageAspectFlags,
    mip_levels: u32,
) -> Result<vk::ImageView> {
    let components = vk::ComponentMapping::builder()
        .r(vk::ComponentSwizzle::IDENTITY)
        .g(vk::ComponentSwizzle::IDENTITY)
        .b(vk::ComponentSwizzle::IDENTITY)
        .a(vk::ComponentSwizzle::IDENTITY);

    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(aspects)
        .base_mip_level(0)
        .level_count(mip_levels)
        .base_array_layer(0)
        .layer_count(1);

    let info = vk::ImageViewCreateInfo::builder()
        .image(image)
        .view_type(vk::ImageViewType::_2D)
        .format(format)
        .subresource_range(subresource_range)
        .components(components);

    let view = unsafe {
        DEVICE
            .create_image_view(&info, None)
            .context("Create image view failed")?
    };
    Ok(view)
}

#[derive(Debug)]
pub struct Image {
    image: vk::Image,
    _alloc: Allocation,
    pub view: vk::ImageView,
    size: vk::Extent3D,
}

impl Image {
    pub fn new(
        size: vk::Extent3D,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        aspects: vk::ImageAspectFlags,
    ) -> Result<Self> {
        let info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::_2D)
            .extent(size)
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(vk::SampleCountFlags::_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let image = unsafe { DEVICE.create_image(&info, None) }.context("Image creation failed")?;
        let requirements = unsafe { DEVICE.get_image_memory_requirements(image) };

        let alloc = allocator()
            .alloc(vk::MemoryPropertyFlags::DEVICE_LOCAL, requirements, false)
            .context("Alloc failed")?;

        unsafe { DEVICE.bind_image_memory(image, alloc.memory(), alloc.offset() as u64) }
            .context("Image memory binding failed")?;

        let view = create_image_view(image, format, aspects, 1)?;

        Ok(Self {
            image,
            _alloc: alloc,
            view,
            size,
        })
    }

    pub fn layout_transition(
        &mut self,
        queue: &Queue,
        command_buff: &mut CommandBuffer,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> Result<()> {
        let (src_access_mask, dst_access_mask, src_stage_mask, dst_stage_mask) =
            match (old_layout, new_layout) {
                (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                ),
                (
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                ) => (
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::AccessFlags::SHADER_READ,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                ),
                _ => bail!("Unsupported image layout transition!"),
            };

        let subresource = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.image)
            .subresource_range(subresource)
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask);

        command_buff.run_one_time_commands(queue, |buff| unsafe {
            DEVICE.cmd_pipeline_barrier(
                buff,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[] as &[vk::MemoryBarrier],
                &[] as &[vk::BufferMemoryBarrier],
                &[barrier],
            );
        })?;

        Ok(())
    }

    pub fn copy_from_buff(&mut self, command_buff: vk::CommandBuffer, buffer: &Buffer) {
        let subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1);

        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(subresource)
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(self.size);

        unsafe {
            DEVICE.cmd_copy_buffer_to_image(
                command_buff,
                buffer.buffer,
                self.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            DEVICE.destroy_image_view(self.view, None);
            DEVICE.destroy_image(self.image, None);
        }
    }
}
