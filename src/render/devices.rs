use std::{ffi::CStr, ops::Deref};

use anyhow::{bail, Context, Result};
use log::{info, warn};
use vulkanalia::{
    vk::{
        self, DeviceCreateInfo, DeviceQueueCreateInfo, DeviceV1_0, HasBuilder, InstanceV1_0,
        KhrSurfaceExtension, PhysicalDeviceProperties, PhysicalDeviceType, QueueFlags,
    },
    Instance,
};

use crate::render::{
    config::VALIDATION_LAYERS,
    queues::{get_queue_families, get_queue_family},
    swapchain::SwapchainSupport,
};

use super::{
    config::{DEVICE_REQUIRED_EXTENSIONS, VALIDATION_ENABLED},
    queues::get_present_queue_family,
};

pub fn pick_physical(instance: &Instance, surface: vk::SurfaceKHR) -> Result<vk::PhysicalDevice> {
    let devices = unsafe {
        instance
            .enumerate_physical_devices()
            .context("Physical devices enumeration failed")?
    };

    let best_device = devices
        .iter()
        .copied()
        .map(|device| {
            let props = unsafe { instance.get_physical_device_properties(device) };
            (device, props)
        })
        .filter(|&(device, props)| {
            let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) };
            let r = filter_device(instance, surface, device, props);
            match r {
                Ok(Ok(())) => true,
                Ok(Err(reason)) => {
                    info!("Device {:?} cannot be used: {}", name, reason);
                    false
                }
                Err(e) => {
                    warn!(
                        "Device {:?} cannot be used: an error occured: {:?}",
                        name, e
                    );
                    false
                }
            }
        })
        .max_by_key(|&(device, props)| score_device(device, props));
    let (device, properties) = match best_device {
        Some(device) => device,
        None => bail!("No suitable physical device"),
    };

    let name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) };
    info!("Selected device is {:?}", name);

    Ok(device)
}

/// Check minimum properties for `device`.
/// Return `Ok(Ok()))` if the device is usable, `Ok(Err(reason))` else and an anyhow error if something went wrong.
fn filter_device(
    instance: &Instance,
    surface: vk::SurfaceKHR,
    device: vk::PhysicalDevice,
    _props: PhysicalDeviceProperties,
) -> anyhow::Result<Result<(), &'static str>> {
    let mut graphics_queue_count = 0;
    let mut present_queue_count = 0;
    for (i, family) in get_queue_families(instance, device).iter().enumerate() {
        if family.queue_flags.intersects(QueueFlags::GRAPHICS) {
            graphics_queue_count += family.queue_count as usize;
        }
        if unsafe { instance.get_physical_device_surface_support_khr(device, i as u32, surface) }
            .context("Unable to query if presentation is supported")?
        {
            present_queue_count += 1;
        }
    }

    if present_queue_count == 0 {
        return Ok(Err("No present queue"));
    }
    if graphics_queue_count == 0 {
        return Ok(Err("No graphics queue"));
    }

    if !check_required_extensions(instance, device)? {
        return Ok(Err("Required extension not found"));
    }

    if !check_swapchain(instance, device, surface)? {
        return Ok(Err("Insufficient swapchain support"));
    }

    Ok(Ok(()))
}

fn check_required_extensions(
    instance: &Instance,
    device: vk::PhysicalDevice,
) -> anyhow::Result<bool> {
    let extensions = unsafe {
        instance
            .enumerate_device_extension_properties(device, None)
            .context("Enumerating device extensions failed")?
    };

    Ok(DEVICE_REQUIRED_EXTENSIONS
        .iter()
        .all(|ext| extensions.iter().any(|&e| e.extension_name == ext.name)))
}

fn check_swapchain(
    instance: &Instance,
    device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
) -> anyhow::Result<bool> {
    let swapchain_support = SwapchainSupport::get(instance, device, surface)
        .context("Querying swapchain support failed")?;
    Ok(!swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty())
}

/// Return a score for the device. The device with the highest score is chosen.
fn score_device(_device: vk::PhysicalDevice, props: PhysicalDeviceProperties) -> isize {
    let mut score = 0;

    score += match props.device_type {
        PhysicalDeviceType::INTEGRATED_GPU => 0,
        PhysicalDeviceType::DISCRETE_GPU => 1000,
        PhysicalDeviceType::VIRTUAL_GPU => -10,
        PhysicalDeviceType::CPU => -100,
        PhysicalDeviceType::OTHER => -100,
        _ => -100,
    };

    score
}

#[derive(Debug)]
pub struct Device {
    pub device: vulkanalia::Device,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
}

impl Deref for Device {
    type Target = vulkanalia::Device;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { self.destroy_device(None) };
    }
}

impl Device {
    pub fn new(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> Result<Self> {
        let graphics_queue_family =
            get_queue_family(instance, physical_device, QueueFlags::GRAPHICS)?
                .context("No graphics queue found")?;
        let present_queue_family = get_present_queue_family(instance, physical_device, surface)?
            .context("No present queue found")?;

        let priority = &[1.0];

        let queue_create_infos = if graphics_queue_family == present_queue_family {
            let info = DeviceQueueCreateInfo::builder()
                .queue_family_index(graphics_queue_family)
                .queue_priorities(priority)
                .build();
            vec![info]
        } else {
            let graphics_info = DeviceQueueCreateInfo::builder()
                .queue_family_index(graphics_queue_family)
                .queue_priorities(priority)
                .build();
            let transfer_info = DeviceQueueCreateInfo::builder()
                .queue_family_index(present_queue_family)
                .queue_priorities(priority)
                .build();
            vec![graphics_info, transfer_info]
        };

        let extensions = DEVICE_REQUIRED_EXTENSIONS
            .iter()
            .map(|ext| ext.name.as_ptr())
            .collect::<Vec<_>>();

        let layers = if VALIDATION_ENABLED {
            VALIDATION_LAYERS
        } else {
            &[]
        };

        let create_info = DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_layer_names(layers)
            .enabled_extension_names(&extensions);

        let device = unsafe { instance.create_device(physical_device, &create_info, None) }?;
        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family, 0) };
        let present_queue = unsafe { device.get_device_queue(present_queue_family, 0) };

        Ok(Self {
            device,
            graphics_queue,
            present_queue,
        })
    }
}
