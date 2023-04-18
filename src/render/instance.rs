use std::{
    ffi::{c_void, CStr},
    ops::Deref,
};

use anyhow::{anyhow, Context, Result};

use log::{debug, error, trace, warn};
use vulkanalia::{
    vk::{
        self, ApplicationInfo, DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT,
        ExtDebugUtilsExtension, HasBuilder, InstanceCreateInfo, InstanceV1_0,
    },
    Entry,
};
use winit::window::Window;

use crate::{
    render::config::{VALIDATION_ENABLED, VALIDATION_LAYERS},
    utils::DerefOnceLock,
};

#[derive(Debug)]
pub struct Instance {
    instance: vulkanalia::Instance,
    debug_messenger: Option<DebugUtilsMessengerEXT>,
}

impl Deref for Instance {
    type Target = vulkanalia::Instance;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            if let Some(messenger) = self.debug_messenger {
                self.instance
                    .destroy_debug_utils_messenger_ext(messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

pub static INSTANCE: DerefOnceLock<Instance, "Instance not initialized"> = DerefOnceLock::new();

impl Instance {
    pub fn init(entry: &Entry, window: &Window) -> Result<()> {
        let instance = Self::new(entry, window)?;
        INSTANCE
            .inner()
            .set(instance)
            .map_err(|_| anyhow!("Instance already initialized"))
    }

    fn new(entry: &Entry, window: &Window) -> Result<Self> {
        let app_version = {
            let major = env!("CARGO_PKG_VERSION_MAJOR")
                .parse()
                .expect("CARGO_PKG_VERSION_MAJOR isn't a number");
            let minor = env!("CARGO_PKG_VERSION_MINOR")
                .parse()
                .expect("CARGO_PKG_VERSION_MINOR isn't a number");
            let patch = env!("CARGO_PKG_VERSION_PATCH")
                .parse()
                .expect("CARGO_PKG_VERSION_PATCH isn't a number");
            vk::make_version(major, minor, patch)
        };
        let app_info = ApplicationInfo::builder()
            .api_version(vk::make_version(1, 2, 0))
            .application_name(b"Vulkan Voxels 2\0")
            .application_version(app_version);

        let layers = if VALIDATION_ENABLED {
            VALIDATION_LAYERS
        } else {
            &[]
        };
        let mut extensions = vulkanalia::window::get_required_instance_extensions(window)
            .iter()
            .map(|&ext| ext.as_ptr())
            .collect::<Vec<_>>();
        if VALIDATION_ENABLED {
            extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr())
        }

        let mut debug_messenger_create_info = DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING, // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .user_callback(Some(debug_callback));

        let instance_create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(layers)
            .enabled_extension_names(&extensions)
            .push_next(&mut debug_messenger_create_info);

        let instance = unsafe { entry.create_instance(&instance_create_info, None) }
            .context("Vulkan instance creation failed")?;

        let debug_messenger = if VALIDATION_ENABLED {
            match unsafe {
                instance.create_debug_utils_messenger_ext(&debug_messenger_create_info, None)
            } {
                Ok(messenger) => Some(messenger),
                Err(e) => {
                    warn!("Debug utils messenger creation failed: {e}");
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            instance,
            debug_messenger,
        })
    }
}

extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        debug!("({:?}) {}", type_, message);
    } else {
        trace!("({:?}) {}", type_, message);
    }

    vk::FALSE
}
