#![allow(dead_code)]

use std::ffi::CStr;
use std::ops::Deref;
use std::slice;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;

use ash::khr::{surface, swapchain};
use ash::vk;

use crossbeam_channel::{Receiver, Sender};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use sdl2::video::Window;

const INSTANCE_EXTENSIONS: &[&CStr] = &[
    ash::ext::debug_utils::NAME,
    ash::khr::surface::NAME,
    ash::ext::surface_maintenance1::NAME,
    ash::khr::get_surface_capabilities2::NAME,
];

const DEVICE_EXTENSIONS: &[&CStr] = &[
    ash::khr::swapchain::NAME,
    ash::ext::swapchain_maintenance1::NAME,
];

#[derive(Clone)]
pub struct Core {
    inner: Arc<CoreInner>,
}

impl Deref for Core {
    type Target = CoreInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct DeferredSubmit {
    pub cmd: vk::CommandBuffer,
}

pub struct CoreInner {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub pdevice: vk::PhysicalDevice,
    pub pdevice_properties: vk::PhysicalDeviceProperties,
    pub pdevice_mem_properties: vk::PhysicalDeviceMemoryProperties,
    pub device: ash::Device,
    pub swapchain_device: swapchain::Device,
    pub surface_instance: surface::Instance,

    pub graphics_queue_family_index: u32,
    pub graphics_queue: vk::Queue,

    pub pipeline_depth: u32,
    pub deferred_submits: (Sender<DeferredSubmit>, Receiver<DeferredSubmit>),

    pub surface: vk::SurfaceKHR,
}

impl Core {
    pub fn new(window: &Window) -> anyhow::Result<Self> {
        let app_info = vk::ApplicationInfo::default().api_version(vk::make_api_version(0, 1, 3, 0));

        let mut extensions = Vec::from_iter(INSTANCE_EXTENSIONS.iter().map(|c| c.as_ptr()));
        extensions.extend_from_slice(ash_window::enumerate_required_extensions(
            window.display_handle()?.as_raw(),
        )?);

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions);

        // Setup Vulkan
        unsafe {
            let entry = ash::Entry::load()?;

            let instance = entry.create_instance(&create_info, None)?;

            let surface = ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )?;

            let surface_instance = surface::Instance::new(&entry, &instance);
            let pdevices = instance.enumerate_physical_devices()?;

            log::info!("Available devices:");

            for &pdevice in &pdevices {
                let properties = instance.get_physical_device_properties(pdevice);
                let name = CStr::from_bytes_until_nul(bytemuck::bytes_of(&properties.device_name))
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("invalid name");

                log::info!("- {name}");
            }

            let (pdevice, queue_family_index) = pdevices
                .iter()
                .find_map(|&pdevice| {
                    instance
                        .get_physical_device_queue_family_properties(pdevice)
                        .iter()
                        .enumerate()
                        .find_map(|(index, info)| {
                            let has_graphics = info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                            let supports_surface = surface_instance
                                .get_physical_device_surface_support(pdevice, index as _, surface)
                                .unwrap_or(false);

                            if has_graphics && supports_surface {
                                Some((pdevice, index as u32))
                            } else {
                                None
                            }
                        })
                })
                .context("could not find suitable physical device")?;

            let pdevice_properties = instance.get_physical_device_properties(pdevice);
            let pdevice_mem_properties = instance.get_physical_device_memory_properties(pdevice);

            let queue_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index)
                .queue_priorities(&[1.0]);

            let extensions = Vec::from_iter(DEVICE_EXTENSIONS.iter().map(|c| c.as_ptr()));

            let mut features_13 = vk::PhysicalDeviceVulkan13Features::default()
                .dynamic_rendering(true)
                .synchronization2(true);

            let mut features_12 = vk::PhysicalDeviceVulkan12Features::default()
                .runtime_descriptor_array(true)
                .descriptor_indexing(true)
                .descriptor_binding_partially_bound(true)
                .descriptor_binding_sampled_image_update_after_bind(true)
                .shader_sampled_image_array_non_uniform_indexing(true)
                .buffer_device_address(true);

            let mut features = vk::PhysicalDeviceFeatures2::default()
                .push_next(&mut features_13)
                .push_next(&mut features_12);

            let device_create_info = vk::DeviceCreateInfo::default()
                .enabled_extension_names(&extensions)
                .queue_create_infos(slice::from_ref(&queue_info))
                .push_next(&mut features);

            let device = instance.create_device(pdevice, &device_create_info, None)?;

            let graphics_queue = device.get_device_queue(queue_family_index, 0);

            let swapchain_device = swapchain::Device::new(&instance, &device);

            let deferred_submits = crossbeam_channel::bounded(16);

            Ok(Self {
                inner: Arc::new(CoreInner {
                    entry,
                    instance,
                    device,
                    pdevice_properties,
                    pdevice_mem_properties,
                    pdevice,
                    swapchain_device,
                    surface_instance,

                    graphics_queue_family_index: queue_family_index,
                    graphics_queue,

                    pipeline_depth: 3,
                    deferred_submits,

                    surface,
                }),
            })
        }
    }

    pub fn graphics_queue_family_index(&self) -> u32 {
        self.inner.graphics_queue_family_index
    }

    pub fn device(&self) -> &ash::Device {
        &self.inner.device
    }

    pub fn graphics_queue(&self) -> vk::Queue {
        self.inner.graphics_queue
    }

    pub fn deferred_submit(&self, cmd: vk::CommandBuffer) {
        let _ = self.deferred_submits.0.send(DeferredSubmit { cmd });
    }

    pub fn cmd_image_barrier(
        &self,
        cmd: vk::CommandBuffer,
        image: vk::Image,
        old: vk::ImageLayout,
        new: vk::ImageLayout,
        aspect: vk::ImageAspectFlags,
    ) {
        let image_barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
            .old_layout(old)
            .new_layout(new)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(aspect)
                    .level_count(vk::REMAINING_MIP_LEVELS)
                    .layer_count(vk::REMAINING_ARRAY_LAYERS),
            )
            .image(image);

        let dep_info =
            vk::DependencyInfo::default().image_memory_barriers(slice::from_ref(&image_barrier));

        unsafe {
            self.device.cmd_pipeline_barrier2(cmd, &dep_info);
        }
    }
}

pub fn seconds(v: u64) -> u64 {
    Duration::from_secs(v).as_nanos() as u64
}

impl Drop for CoreInner {
    fn drop(&mut self) {
        unsafe {
            if self.device.device_wait_idle().is_err() {
                return;
            }

            surface::Instance::new(&self.entry, &self.instance).destroy_surface(self.surface, None);

            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
