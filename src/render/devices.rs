use std::{ffi::CStr, ops::Deref};

use anyhow::{anyhow, bail, Context, Result};
use log::{info, warn};
use vulkanalia::vk::{
    self, DeviceCreateInfo, DeviceV1_0, HasBuilder, InstanceV1_0, KhrSurfaceExtension,
    PhysicalDeviceProperties, PhysicalDeviceType, QueueFlags,
};

use crate::{
    render::{config::VALIDATION_LAYERS, instance::INSTANCE, swapchain::SwapchainSupport},
    utils::DerefOnceLock,
};

use super::{
    config::{DEVICE_REQUIRED_EXTENSIONS, VALIDATION_ENABLED},
    queues::{get_queue_families, QueuesManager, QUEUES},
};

pub fn pick_physical(surface: vk::SurfaceKHR) -> Result<vk::PhysicalDevice> {
    let devices = unsafe {
        INSTANCE
            .enumerate_physical_devices()
            .context("Physical devices enumeration failed")?
    };

    let best_device = devices
        .iter()
        .copied()
        .map(|device| {
            let props = unsafe { INSTANCE.get_physical_device_properties(device) };
            (device, props)
        })
        .filter(|&(device, props)| {
            let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) };
            let r = filter_device(surface, device, props);
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
    info!("Selected device {:?}", name);

    Ok(device)
}

/// Check minimum properties for `device`.
/// Return `Ok(Ok()))` if the device is usable, `Ok(Err(reason))` else and an anyhow error if something went wrong.
fn filter_device(
    surface: vk::SurfaceKHR,
    device: vk::PhysicalDevice,
    _props: PhysicalDeviceProperties,
) -> anyhow::Result<Result<(), &'static str>> {
    let mut graphics_queue_count = 0;
    let mut present_queue_count = 0;
    for (i, family) in get_queue_families(device).iter().enumerate() {
        if family.queue_flags.intersects(QueueFlags::GRAPHICS) {
            graphics_queue_count += family.queue_count as usize;
        }
        if unsafe { INSTANCE.get_physical_device_surface_support_khr(device, i as u32, surface) }
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

    if !check_required_extensions(device)? {
        return Ok(Err("Required extension not found"));
    }

    if !check_swapchain(device, surface)? {
        return Ok(Err("Insufficient swapchain support"));
    }

    Ok(Ok(()))
}

fn check_required_extensions(device: vk::PhysicalDevice) -> anyhow::Result<bool> {
    let extensions = unsafe {
        INSTANCE
            .enumerate_device_extension_properties(device, None)
            .context("Enumerating device extensions failed")?
    };

    Ok(DEVICE_REQUIRED_EXTENSIONS
        .iter()
        .all(|ext| extensions.iter().any(|&e| e.extension_name == ext.name)))
}

fn check_swapchain(device: vk::PhysicalDevice, surface: vk::SurfaceKHR) -> anyhow::Result<bool> {
    let swapchain_support =
        SwapchainSupport::get(device, surface).context("Querying swapchain support failed")?;
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

pub static DEVICE: DerefOnceLock<Device, "Device not initialized"> = DerefOnceLock::new();

#[derive(Debug)]
pub struct Device {
    pub device: vulkanalia::Device,
    pub graphics_queue: vk::Queue,
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
    #[inline]
    pub fn init(physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR) -> Result<()> {
        let device = Self::new(physical_device, surface)?;
        DEVICE
            .inner()
            .set(device)
            .map_err(|_| anyhow!("Device already initialized"))
    }

    fn new(physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR) -> Result<Self> {
        let (_priorities, queue_create_infos) = QueuesManager::init(physical_device, surface)?;

        let extensions = DEVICE_REQUIRED_EXTENSIONS
            .iter()
            .map(|ext| ext.name.as_ptr())
            .collect::<Vec<_>>();

        let layers = if VALIDATION_ENABLED {
            VALIDATION_LAYERS
        } else {
            &[]
        };

        let features = vk::PhysicalDeviceFeatures::builder()
            .shader_int64(true)
            .fill_mode_non_solid(true);
        let mut features12 = vk::PhysicalDeviceVulkan12Features::builder().shader_int8(true);
        let create_info = DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_layer_names(layers)
            .enabled_extension_names(&extensions)
            .enabled_features(&features)
            .push_next(&mut features12);

        let graphics_queue_info = QUEUES.get_default_graphics();
        // Safety: _priorities is dropped after this
        let device = unsafe { INSTANCE.create_device(physical_device, &create_info, None) }?;
        let graphics_queue = unsafe {
            device.get_device_queue(graphics_queue_info.family, graphics_queue_info.index)
        };

        Ok(Self {
            device,
            graphics_queue,
        })
    }
}
