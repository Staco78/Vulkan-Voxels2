use anyhow::Result;
use vulkanalia::{
    vk::{self, InstanceV1_0, KhrSurfaceExtension, QueueFamilyProperties, QueueFlags, SurfaceKHR},
    Instance,
};

#[inline]
pub fn get_queue_families(
    instance: &Instance,
    device: vk::PhysicalDevice,
) -> Vec<QueueFamilyProperties> {
    unsafe { instance.get_physical_device_queue_family_properties(device) }
}

pub fn get_queue_family_filtered<F>(
    instance: &Instance,
    device: vk::PhysicalDevice,
    family_type: QueueFlags,
    filter: F,
) -> Result<Option<u32>>
where
    F: Fn(u32, &QueueFamilyProperties) -> bool,
{
    let families = get_queue_families(instance, device);
    let (index, _) = families
        .iter()
        .copied()
        .enumerate()
        .map(|(i, f)| (i as u32, f))
        .filter(|(_, family)| family.queue_flags.intersects(family_type))
        .find(|(i, family)| filter(*i, family))
        .unzip();

    Ok(index)
}

#[inline]
pub fn get_queue_family(
    instance: &Instance,
    device: vk::PhysicalDevice,
    family_type: QueueFlags,
) -> Result<Option<u32>> {
    get_queue_family_filtered(instance, device, family_type, |_, _| true)
}

pub fn get_present_queue_family(
    instance: &Instance,
    device: vk::PhysicalDevice,
    surface: SurfaceKHR,
) -> Result<Option<u32>> {
    get_queue_family_filtered(instance, device, QueueFlags::all(), |i, _| unsafe {
        instance
            .get_physical_device_surface_support_khr(device, i, surface)
            .unwrap_or(false)
    })
}
