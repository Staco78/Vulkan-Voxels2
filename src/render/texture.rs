use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use crate::render::DEVICE;

use super::{descriptors::DescriptorSet, image::Image, Buffer, CommandBuffer};

#[derive(Debug)]
pub struct TextureCreationOptions {
    pub format: vk::Format,
    pub filter: vk::Filter,
    pub address_mode: vk::SamplerAddressMode,
    pub anisotropy: bool,
}

impl Default for TextureCreationOptions {
    fn default() -> Self {
        Self {
            format: vk::Format::R8G8B8A8_SRGB,
            filter: vk::Filter::LINEAR,
            address_mode: vk::SamplerAddressMode::REPEAT,
            anisotropy: true,
        }
    }
}

#[derive(Debug)]
pub struct Texture {
    _image: Image,
    sampler: vk::Sampler,
    pub descriptor_set: DescriptorSet,
}

impl Texture {
    /// Create a new texture by copying pixels from `buff`. Buff should have been created with `TRANSFER_SRC`.
    pub fn new(
        command_buff: &mut CommandBuffer,
        buff: &Buffer,
        size: vk::Extent3D,
        binding: u32,
        mut descriptor_set: DescriptorSet,
        options: &TextureCreationOptions,
    ) -> Result<Self> {
        let mut image = Image::new(
            size,
            options.format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::ImageAspectFlags::COLOR,
        )
        .context("Image creation failed")?;

        image
            .layout_transition(
                &DEVICE.graphics_queue,
                command_buff,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            )
            .context("Image layout transition failed")?;

        command_buff
            .run_one_time_commands(&DEVICE.graphics_queue, |cmd_buff| {
                image.copy_from_buff(cmd_buff, buff);
            })
            .context("Image copy from buffer failed")?;

        image
            .layout_transition(
                &DEVICE.graphics_queue,
                command_buff,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            )
            .context("Image layout transition failed")?;

        let info = vk::SamplerCreateInfo::builder()
            .mag_filter(options.filter)
            .min_filter(options.filter)
            .address_mode_u(options.address_mode)
            .address_mode_v(options.address_mode)
            .address_mode_w(options.address_mode)
            .anisotropy_enable(options.anisotropy)
            .max_anisotropy(16.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .min_lod(0.0)
            .max_lod(vk::LOD_CLAMP_NONE);
        let sampler =
            unsafe { DEVICE.create_sampler(&info, None) }.context("Sampler creation failed")?;

        let info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(image.view)
            .sampler(sampler);
        let image_info = &[info];
        let sampler_write = vk::WriteDescriptorSet::builder()
            .dst_set(*descriptor_set)
            .dst_binding(binding)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(image_info);
        descriptor_set.update(&[sampler_write]);

        Ok(Self {
            _image: image,
            sampler,
            descriptor_set,
        })
    }

    pub fn binding(binding: u32) -> vk::DescriptorSetLayoutBinding {
        vk::DescriptorSetLayoutBinding::builder()
            .binding(binding)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build()
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { DEVICE.destroy_sampler(self.sampler, None) };
    }
}
