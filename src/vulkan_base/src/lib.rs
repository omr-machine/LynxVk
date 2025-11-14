mod vulkan_base;

use vulkan_base::*;

use ash::extensions::khr;
use ash::vk;
use scopeguard::{guard, ScopeGuard};

pub struct VulkanBase {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub surface_loader: khr::Surface,
    pub debug_utils_loader: ash::extensions::ext::DebugUtils,
    pub surface: vk::SurfaceKHR,
    pub physical_device: vk::PhysicalDevice,
    pub physical_device_properties: vk::PhysicalDeviceProperties,
    pub surface_format: vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
    pub depth_format: vk::Format,
    pub queue_family: u32,
    pub device: ash::Device,
    pub queue: vk::Queue,
}

impl VulkanBase {
    pub fn new<'a, 'b>(
        window: &winit::window::Window,
        required_instance_extensions: &Vec<&'a std::ffi::CStr>,
        required_device_extensions: &Vec<&'b std::ffi::CStr>,
    ) -> Result<Self, String> {
        let entry = create_entry();
        check_instance_version(&entry)?;
        check_required_instance_extensions(&entry, required_instance_extensions)?;

        let instance_sg = {
            let instance = create_instance(&entry, required_instance_extensions)?;
            guard(instance, |instance| {
                log::warn!("instance scopeguard");
                unsafe {
                    instance.destroy_instance(None);
                }
            })
        };

        let debug_utils_loader = create_debug_utils_loader(&entry, &instance_sg);
        let surface_loader = create_surface_loader(&entry, &instance_sg);

        let surface_sg = {
            let surface = create_surface(&entry, &instance_sg, window)?;
            guard(surface, |surface| {
                log::warn!("surface scopeguard");
                unsafe {
                    surface_loader.destroy_surface(surface, None);
                }
            })
        };

        let physical_device = get_physical_device(&instance_sg, &required_device_extensions)?;
        let physical_device_properties =
            get_physical_device_properties(&instance_sg, physical_device);
        let surface_format = get_surface_format(physical_device, &surface_loader, *surface_sg)?;
        let present_mode = get_present_mode(physical_device, &surface_loader, *surface_sg)?;
        let queue_family =
            get_queue_family(&instance_sg, physical_device, &surface_loader, *surface_sg)?;
        let depth_format = get_depth_format(&instance_sg, physical_device)?;

        let device_sg = {
            let device = create_logical_device(
                &instance_sg,
                physical_device,
                queue_family,
                &required_device_extensions,
            )?;
            guard(device, |device| {
                log::warn!("device scopeguard");
                unsafe {
                    device.destroy_device(None);
                }
            })
        };

        let queue = get_queue(&device_sg, queue_family);

        Ok(VulkanBase {
            entry,
            instance: ScopeGuard::into_inner(instance_sg),
            surface: ScopeGuard::into_inner(surface_sg),
            surface_loader,
            debug_utils_loader,
            physical_device,
            physical_device_properties,
            surface_format,
            present_mode,
            depth_format,
            queue_family,
            device: ScopeGuard::into_inner(device_sg),
            queue,
        })
    }

    pub fn clean(self) {
        log::info!("cleaning vulkan base");

        unsafe {
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}
