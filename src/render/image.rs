use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use crate::render::memory::allocator;

use super::{devices::DEVICE, memory::Allocation};

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
}

impl Image {
    pub fn new(
        size: vk::Extent2D,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        aspects: vk::ImageAspectFlags,
    ) -> Result<Self> {
        let info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::_2D)
            .extent(vk::Extent3D {
                width: size.width,
                height: size.height,
                depth: 1,
            })
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

        unsafe { DEVICE.bind_image_memory(image, alloc.memory(), 0) }
            .context("Image memory binding failed")?;

        let view = create_image_view(image, format, aspects, 1)?;

        Ok(Self {
            image,
            _alloc: alloc,
            view,
        })
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
