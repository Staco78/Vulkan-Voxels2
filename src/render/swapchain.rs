use anyhow::{Context, Result};
use vulkanalia::vk::{
    self, DeviceV1_0, Handle, HasBuilder, KhrSurfaceExtension, KhrSwapchainExtension, SurfaceKHR,
};
use winit::window::Window;

use crate::utils::drop_then_new;

use super::{devices::DEVICE, image::create_image_view, instance::INSTANCE};

#[derive(Clone, Debug)]
pub struct SwapchainSupport {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupport {
    pub fn get(device: vk::PhysicalDevice, surface: SurfaceKHR) -> Result<Self> {
        unsafe {
            Ok(Self {
                capabilities: INSTANCE
                    .get_physical_device_surface_capabilities_khr(device, surface)
                    .context("Querying surface capabilities failed")?,
                formats: INSTANCE
                    .get_physical_device_surface_formats_khr(device, surface)
                    .context("Querying surface formats failed")?,
                present_modes: INSTANCE
                    .get_physical_device_surface_present_modes_khr(device, surface)
                    .context("Querying surface present modes failed")?,
            })
        }
    }

    pub fn get_best_format(&self) -> vk::SurfaceFormatKHR {
        self.formats
            .iter()
            .cloned()
            .find(|f| {
                f.format == vk::Format::B8G8R8A8_SRGB
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or_else(|| self.formats[0])
    }

    #[cfg(not(feature = "bench"))]
    #[inline]
    pub fn get_best_present_mode(&self) -> vk::PresentModeKHR {
        vk::PresentModeKHR::FIFO
    }

    #[cfg(feature = "bench")]
    #[inline]
    pub fn get_best_present_mode(&self) -> vk::PresentModeKHR {
        vk::PresentModeKHR::MAILBOX
    }

    fn get_extent(&self, window: &Window) -> vk::Extent2D {
        if self.capabilities.current_extent.width != u32::MAX {
            self.capabilities.current_extent
        } else {
            let size = window.inner_size();
            vk::Extent2D::builder()
                .width(u32::clamp(
                    self.capabilities.min_image_extent.width,
                    self.capabilities.max_image_extent.width,
                    size.width,
                ))
                .height(u32::clamp(
                    self.capabilities.min_image_extent.height,
                    self.capabilities.max_image_extent.height,
                    size.height,
                ))
                .build()
        }
    }
}

#[derive(Debug)]
pub struct Swapchain {
    pub swapchain: vk::SwapchainKHR,
    pub format: vk::SurfaceFormatKHR,
    pub extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

impl Swapchain {
    pub fn new(
        physical_device: vk::PhysicalDevice,
        window: &Window,
        surface: SurfaceKHR,
    ) -> Result<Self> {
        let support = SwapchainSupport::get(physical_device, surface)
            .context("Querying swapchain support failed")?;

        let format = support.get_best_format();
        let present_mode = support.get_best_present_mode();
        let extent = support.get_extent(window);

        let mut image_count = support.capabilities.min_image_count + 1;
        if support.capabilities.max_image_count != 0
            && image_count > support.capabilities.max_image_count
        {
            image_count = support.capabilities.max_image_count;
        }

        let info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&[])
            .pre_transform(support.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());

        let swapchain = unsafe { DEVICE.create_swapchain_khr(&info, None)? };
        let images = unsafe { DEVICE.get_swapchain_images_khr(swapchain)? };
        let image_views = images
            .iter()
            .map(|i| create_image_view(*i, format.format, vk::ImageAspectFlags::COLOR, 1))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            swapchain,
            format,
            extent,
            images,
            image_views,
        })
    }

    #[inline]
    pub fn recreate(
        &mut self,
        physical_device: vk::PhysicalDevice,
        window: &Window,
        surface: SurfaceKHR,
    ) -> Result<()> {
        drop_then_new(self, || Self::new(physical_device, window, surface))
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for view in &self.image_views {
                DEVICE.destroy_image_view(*view, None);
            }
            DEVICE.destroy_swapchain_khr(self.swapchain, None)
        };
    }
}
