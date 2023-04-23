use anyhow::{Context, Result};
use vulkanalia::vk::{self, InstanceV1_0};

use crate::utils::drop_then_new;

use super::{image::Image, instance::INSTANCE, swapchain::Swapchain};

#[derive(Debug)]
pub struct DepthBuffer {
    image: Image,
}

impl DepthBuffer {
    pub fn new(physical_device: vk::PhysicalDevice, swapchain: &Swapchain) -> Result<Self> {
        let image = Image::new(
            swapchain.extent,
            Self::get_format(physical_device).context("No supported format found")?,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::ImageAspectFlags::DEPTH,
        )
        .context("Image creation failed")?;
        Ok(Self { image })
    }

    pub fn recreate(
        &mut self,
        physical_device: vk::PhysicalDevice,
        swapchain: &Swapchain,
    ) -> Result<()> {
        drop_then_new(self, || Self::new(physical_device, swapchain))
    }

    #[inline(always)]
    pub fn view(&self) -> vk::ImageView {
        self.image.view
    }

    pub fn get_format(physical_device: vk::PhysicalDevice) -> Option<vk::Format> {
        let formats = [
            vk::Format::D16_UNORM,
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ];

        formats.iter().copied().find(|&format| {
            let properties =
                unsafe { INSTANCE.get_physical_device_format_properties(physical_device, format) };
            properties
                .optimal_tiling_features
                .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
        })
    }
}
