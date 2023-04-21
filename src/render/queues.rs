use std::{ops::Deref, sync::Mutex};

use anyhow::{anyhow, Context, Result};
use vulkanalia::vk::{
    self, DeviceV1_0, HasBuilder, InstanceV1_0, KhrSurfaceExtension, QueueFamilyProperties,
};

use crate::utils::DerefOnceLock;

use super::{devices::DEVICE, instance::INSTANCE};

#[inline]
pub fn get_queue_families(device: vk::PhysicalDevice) -> Vec<QueueFamilyProperties> {
    unsafe { INSTANCE.get_physical_device_queue_family_properties(device) }
}

#[derive(Debug, Clone, Copy)]
pub struct QueueInfo {
    pub family: u32,
    pub index: u32,
}

#[derive(Debug)]
pub struct Queue {
    inner: vk::Queue,
    pub family: u32,
    pub index: u32,
}

impl Deref for Queue {
    type Target = vk::Queue;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
struct QueueFamilyInfo {
    index: u32,
    flags: vk::QueueFlags,
    count: u32,
    offset: u32,
}

const GRAPHICS_COUNT: usize = 1;
const TRANSFER_COUNT: usize = 1;

pub static QUEUES: DerefOnceLock<QueuesManager, "Queues manager not initialized"> =
    DerefOnceLock::new();

#[derive(Debug)]
pub struct QueuesManager {
    families: Mutex<Vec<QueueFamilyInfo>>,
    graphics: QueueInfo,
}

impl QueuesManager {
    pub fn init(
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> Result<Vec<vk::DeviceQueueCreateInfo>> {
        let (queues, create_infos) = Self::new(physical_device, surface)?;
        QUEUES
            .inner()
            .set(queues)
            .map_err(|_| anyhow!("Queues manager already initialized"))?;
        Ok(create_infos)
    }

    fn new(
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> Result<(Self, Vec<vk::DeviceQueueCreateInfo>)> {
        let families = get_queue_families(physical_device);
        let mut selected_families = vec![];
        let mut found_graphics = 0;
        let mut found_transfer = 0;

        for (i, family) in families.iter().enumerate() {
            let mut count = family.queue_count as usize;
            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                let found_count = (GRAPHICS_COUNT - found_graphics).min(count);
                found_graphics += found_count;
                count -= found_count;
            }
            if family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                let found_count = (TRANSFER_COUNT - found_transfer).min(count);
                found_transfer += found_count;
                count -= found_count;
            }

            let used_count = family.queue_count - count as u32;
            if used_count > 0 {
                selected_families.push((i as u32, used_count, family));
            }
        }

        let max_queue_count = selected_families
            .iter()
            .map(|&(_, count, _)| count)
            .max()
            .unwrap_or(0);
        let priorities = vec![1.0; max_queue_count as usize];
        let create_infos = selected_families
            .iter()
            .map(|&(index, count, _)| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(index)
                    .queue_priorities(&priorities[..count as usize])
                    .build()
            })
            .collect::<Vec<_>>();

        let mut families = selected_families
            .iter()
            .map(|&(i, count, queue)| QueueFamilyInfo {
                index: i,
                flags: queue.queue_flags,
                count,
                offset: 0,
            })
            .collect::<Vec<_>>();
        let first_graphics_family = families
            .iter()
            .find(|&queue| {
                queue.flags.contains(vk::QueueFlags::GRAPHICS)
                    && unsafe {
                        INSTANCE
                            .get_physical_device_surface_support_khr(
                                physical_device,
                                queue.index,
                                surface,
                            )
                            .unwrap_or(false)
                    }
            })
            .context("No graphics queue family")?
            .index;
        let graphics = QueueInfo {
            family: first_graphics_family,
            index: 0,
        };
        families[first_graphics_family as usize].offset = 1;

        Ok((
            Self {
                families: Mutex::new(families),
                graphics,
            },
            create_infos,
        ))
    }

    /// Get the info for the graphics queue used for rendering and presenting.
    #[inline]
    pub fn get_default_graphics(&self) -> QueueInfo {
        self.graphics
    }

    pub fn fetch_queue(&self, family_type: vk::QueueFlags) -> Result<Queue> {
        let mut families = self.families.lock().expect("Mutex poisoned");
        let (i, _) = families
            .iter()
            .enumerate()
            .find(|&(_, queue)| queue.flags.contains(family_type) && queue.count - queue.offset > 0)
            .with_context(|| format!("No unused {:?} queue found", family_type))?;
        let family_info = &mut families[i];
        let index = family_info.offset;
        family_info.offset += 1;
        drop(families);

        let queue = unsafe { DEVICE.get_device_queue(i as u32, index) };

        Ok(Queue {
            inner: queue,
            family: i as u32,
            index,
        })
    }
}
