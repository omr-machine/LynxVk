use ash::extensions::ext;
use ash::extensions::khr;
use ash::vk;
use gpu_allocator::vulkan;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use scopeguard::{guard, ScopeGuard};

use crate::vulkan_utils::MemImage;

pub fn compatibility_check<'a>(
    entry: &ash::Entry,
    required_instance_extensions: &Vec<&'a std::ffi::CStr>,
) -> Result<(), String> {
    // api version
    let api_version = if let Ok(result) = entry.try_enumerate_instance_version() {
        match result {
            Some(version) => version,
            None => vk::make_api_version(0, 1, 0, 0),
        }
    } else {
        return Err(String::from("failed to enumerate instance version"));
    };

    log::info!(
        "instance version: {}.{}.{}",
        vk::api_version_major(api_version),
        vk::api_version_minor(api_version),
        vk::api_version_patch(api_version)
    );

    if vk::api_version_major(api_version) < 1 && vk::api_version_minor(api_version) < 2 {
        return Err(String::from(
            "minimum supported vulkan api version is 1.2.0",
        ));
    }

    let supported_instance_extensions =
        entry.enumerate_instance_extension_properties(None).unwrap();

    let mut supported_instance_extensions_set = std::collections::HashSet::new();
    for vk::ExtensionProperties { extension_name, .. } in &supported_instance_extensions {
        supported_instance_extensions_set
            .insert(unsafe { std::ffi::CStr::from_ptr(extension_name.as_ptr()) });
    }

    for &extension_name in required_instance_extensions {
        if !supported_instance_extensions_set.contains(extension_name) {
            return Err(format!(
                "instance extension {:?} is not supported",
                extension_name
            ));
        }
    }

    Ok(())
    // return Err(format!("TEST ERROR"));
}

// pub fn compatibility_check2() -> {

// }

pub struct VulkanBase {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub surface_loader: khr::Surface,
    pub swapchain_loader: khr::Swapchain,
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
    pub allocator: gpu_allocator::vulkan::Allocator,
    pub surface_capabilities: vk::SurfaceCapabilitiesKHR,
    pub surface_extent: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub depth_buffer_mem_image: MemImage,
}

impl VulkanBase {
    pub fn new<'a, 'b>(
        window: &winit::window::Window,
        required_instance_extensions: &Vec<&'a std::ffi::CStr>,
        required_device_extensions: &Vec<&'b std::ffi::CStr>,
    ) -> Result<Self, String> {
        let entry = ash::Entry::linked();

        match compatibility_check(&entry, required_instance_extensions) {
            Ok(_) => log::info!("compatibility check passed"),
            Err(_) => {
                return Err(String::from("compatibility check failed"));
            }
        };

        let extension_names_raw = required_instance_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        let app_info = vk::ApplicationInfo::builder()
            .api_version(vk::make_api_version(0, 1, 2, 0))
            .build();

        let create_info = vk::InstanceCreateInfo::builder()
            .enabled_extension_names(&extension_names_raw)
            .application_info(&app_info)
            .build();

        let instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .map_err(|_| String::from("failed to create instance"))?
        };

        let debug_utils_loader = ext::DebugUtils::new(&entry, &instance);
        let surface_loader = khr::Surface::new(&entry, &instance);

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
            .map_err(|_| String::from("failed to create surface"))?
        };

        let physical_device = get_physical_device(&instance, &required_device_extensions)?;

        let physical_device_properties =
            unsafe { instance.get_physical_device_properties(physical_device) };

        let formats = match unsafe {
            surface_loader.get_physical_device_surface_formats(physical_device, surface)
        } {
            Ok(formats) => formats,
            Err(_) => {
                return Err(String::from(
                    "failed to get physical device surface formats",
                ));
            }
        };

        let mut found_surface_format = false;
        let mut surface_format = vk::SurfaceFormatKHR {
            format: vk::Format::B8G8R8A8_UNORM,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        };
        for f in &formats {
            if f.format == vk::Format::B8G8R8A8_UNORM
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                surface_format = vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8A8_UNORM,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                };
                found_surface_format = true;
            } else {
                continue;
            }
        }

        if (found_surface_format) {
            log::info!("found surface formats");
        } else {
            return Err(String::from("cannot find surface format"));
        }

        let mut present_mode = vk::PresentModeKHR::FIFO;

        let modes = match unsafe {
            surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
        } {
            Ok(formats) => formats,
            Err(_) => {
                return Err(String::from(
                    "failed to get physical device surface present modes",
                ));
            }
        };

        if modes.is_empty() {
            return Err(String::from(
                "failed to get physical device surface present modes",
            ));
        }

        if modes.contains(&vk::PresentModeKHR::IMMEDIATE) {
            present_mode = vk::PresentModeKHR::IMMEDIATE;
        }

        if modes.contains(&vk::PresentModeKHR::MAILBOX) {
            present_mode = vk::PresentModeKHR::MAILBOX;
        }

        log::info!("selected present mode: {:?}", present_mode);

        let mut queue_family = 0u32;

        let props =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let mut found_queue_with_support = false;
        for (ind, p) in props.iter().enumerate() {
            if p.queue_count > 0 && p.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                let present_supported = match unsafe {
                    surface_loader.get_physical_device_surface_support(
                        physical_device,
                        ind as u32,
                        surface,
                    )
                } {
                    Ok(result) => result,
                    Err(_) => {
                        return Err(String::from(
                            "failed to get physical device surface_support",
                        ));
                    }
                };

                if present_supported {
                    queue_family = ind as u32;
                    found_queue_with_support = true;
                    break;
                }
            }
        }

        if (found_queue_with_support) {
            log::info!("selected queue family: {}", queue_family);
        } else {
            return Err(String::from(
                "failed to find graphics queue with present support",
            ));
        }

        let mut depth_format = vk::Format::D16_UNORM_S8_UINT;

        let format_candidates = [
            vk::Format::D16_UNORM_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
            vk::Format::D32_SFLOAT_S8_UINT,
        ];

        for &format_depth in &format_candidates {
            let props = unsafe {
                instance.get_physical_device_format_properties(physical_device, format_depth)
            };

            if props
                .optimal_tiling_features
                .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
            {
                depth_format = format_depth;
            }
        }

        let queue_indices = [queue_family];

        let mut queue_priorities = Vec::new();
        for _ in &queue_indices {
            queue_priorities.push(vec![1.0f32])
        }

        let mut queue_create_infos = Vec::with_capacity(queue_indices.len());

        for (ind, &family_index) in queue_indices.iter().enumerate() {
            let info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(family_index)
                .queue_priorities(&queue_priorities[ind]);

            queue_create_infos.push(info.build());
        }

        let features = vk::PhysicalDeviceFeatures::builder()
            .tessellation_shader(true)
            .fill_mode_non_solid(true)
            .build();

        let device_extensions_raw = required_device_extensions
            .iter()
            .map(|&s| s.as_ptr())
            .collect::<Vec<*const std::os::raw::c_char>>();

        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions_raw)
            .enabled_features(&features);

        let device = unsafe {
            instance
                .create_device(physical_device, &create_info, None)
                .map_err(|_| String::from("failed to create device"))?
        };

        let queue = unsafe { device.get_device_queue(queue_family, 0) };

        let debug_settings = gpu_allocator::AllocatorDebugSettings {
            log_memory_information: true,
            log_leaks_on_shutdown: true,
            store_stack_traces: false,
            log_allocations: true,
            log_frees: true,
            log_stack_traces: false,
        };

        let create_info = &vulkan::AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device,
            debug_settings,
            buffer_device_address: false,
        };

        let mut allocator = vulkan::Allocator::new(&create_info)
            .map_err(|_| String::from("failed to create allocator"))?;

        let swapchain_loader = khr::Swapchain::new(&instance, &device);

        let resize_data = resize_internal(
            window,
            &device,
            &surface_loader,
            &swapchain_loader,
            physical_device,
            vk::SwapchainKHR::null(),
            &surface,
            &surface_format,
            present_mode,
            &vec![],
            depth_format,
            &mut allocator,
            None,
        )?;

        Ok(VulkanBase {
            entry,
            instance,
            surface,
            surface_loader,
            debug_utils_loader,
            physical_device,
            physical_device_properties,
            surface_format,
            present_mode,
            depth_format,
            queue_family,
            queue,
            allocator,
            surface_capabilities: resize_data.surface_capabilities,
            surface_extent: resize_data.surface_extent,
            swapchain: resize_data.swapchain,
            swapchain_images: resize_data.swapchain_images,
            swapchain_image_views: resize_data.swapchain_image_views,
            swapchain_loader,
            device,
            depth_buffer_mem_image: resize_data.depth_buffer_mem_image,
        })
    }

    pub fn resize(&mut self, window: &winit::window::Window) -> Result<(), String> {
        let old_depth_buffer_mem_image = std::mem::take(&mut self.depth_buffer_mem_image);
        let resize_data = resize_internal(
            window,
            &self.device,
            &self.surface_loader,
            &self.swapchain_loader,
            self.physical_device,
            self.swapchain,
            &self.surface,
            &self.surface_format,
            self.present_mode,
            &self.swapchain_image_views,
            self.depth_format,
            &mut self.allocator,
            Some(old_depth_buffer_mem_image),
        )?;

        self.surface_capabilities = resize_data.surface_capabilities;
        self.surface_extent = resize_data.surface_extent;
        self.swapchain = resize_data.swapchain;
        self.swapchain_images = resize_data.swapchain_images;
        self.swapchain_image_views = resize_data.swapchain_image_views;
        self.depth_buffer_mem_image = resize_data.depth_buffer_mem_image;

        Ok(())
    }

    pub fn clean(mut self) {
        log::info!("cleaning vulkan base");

        unsafe {
            self.device
                .destroy_image(self.depth_buffer_mem_image.image, None);
            self.device
                .destroy_image_view(self.depth_buffer_mem_image.view, None);
            let _ = self.allocator.free(self.depth_buffer_mem_image.allocation);
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            for &image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(image_view, None);
            }
            drop(self.allocator);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

struct ResizeResult {
    surface_capabilities: vk::SurfaceCapabilitiesKHR,
    surface_extent: vk::Extent2D,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    depth_buffer_mem_image: MemImage,
}

fn resize_internal(
    window: &winit::window::Window,
    device: &ash::Device,
    surface_loader: &ash::extensions::khr::Surface,
    swapchain_loader: &ash::extensions::khr::Swapchain,
    physical_device: vk::PhysicalDevice,
    old_swapchain: vk::SwapchainKHR,
    surface: &vk::SurfaceKHR,
    surface_format: &vk::SurfaceFormatKHR,
    present_mode: vk::PresentModeKHR,
    old_swapchain_image_views: &Vec<vk::ImageView>,
    depth_format: vk::Format,
    allocator: &mut gpu_allocator::vulkan::Allocator,
    old_depth_buffer_mem_image: Option<MemImage>,
) -> Result<ResizeResult, String> {
    log::info!("resizing VulkanBase");

    unsafe {
        let _ = device.device_wait_idle();
    }

    let surface_capabilities = unsafe {
        surface_loader
            .get_physical_device_surface_capabilities(physical_device, *surface)
            .map_err(|_| String::from("failed to get physical device surface capabilities"))?
    };

    let window_size = window.inner_size();
    let mut surface_extent = vk::Extent2D::default();

    if surface_capabilities.current_extent.width == u32::MAX {
        surface_extent.width = std::cmp::max(
            surface_capabilities.min_image_extent.width,
            std::cmp::min(
                surface_capabilities.max_image_extent.width,
                window_size.width,
            ),
        );
        surface_extent.height = std::cmp::max(
            surface_capabilities.min_image_extent.height,
            std::cmp::min(
                surface_capabilities.max_image_extent.height,
                window_size.height,
            ),
        );
    } else {
        surface_extent = surface_capabilities.current_extent;
    }

    let surface_extent = surface_extent;

    let mut image_count = std::cmp::max(surface_capabilities.min_image_count, 3);

    if surface_capabilities.max_image_count != 0 {
        image_count = std::cmp::min(image_count, surface_capabilities.max_image_count);
    }

    let create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(*surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(surface_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(surface_capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(old_swapchain)
        .build();

    let swapchain = unsafe {
        swapchain_loader
            .create_swapchain(&create_info, None)
            .map_err(|_| String::from("failed to create swapchain"))?
    };

    if old_swapchain != vk::SwapchainKHR::null() {
        unsafe { swapchain_loader.destroy_swapchain(old_swapchain, None) };
    }

    // no need to explicitly destroy images. They are destroyed when the swapchain is destroyed.
    let swapchain_images = unsafe {
        swapchain_loader
            .get_swapchain_images(swapchain)
            .map_err(|_| String::from("failed to get swapchain images"))?
    };

    if !old_swapchain_image_views.is_empty() {
        log::info!("destroying old swapchain image views");
        for &image_view in old_swapchain_image_views {
            unsafe {
                device.destroy_image_view(image_view, None);
            };
        }
    }

    let swapchain_image_view_sgs = {
        let swapchain_image_views =
            create_swapchain_image_views(device, &swapchain_images, surface_format)?;

        let mut sgs = Vec::with_capacity(swapchain_image_views.len());
        for (i, &image_view) in swapchain_image_views.iter().enumerate() {
            let sg = guard(image_view, move |image_view| {
                log::warn!("swapchain image view {} scopeguard", i);
                unsafe {
                    device.destroy_image_view(image_view, None);
                }
            });
            sgs.push(sg);
        }

        sgs
    };

    if let Some(mem_image) = old_depth_buffer_mem_image {
        log::info!("destroying old depth buffer");
        unsafe {
            device.destroy_image(mem_image.image, None);
            device.destroy_image_view(mem_image.view, None);
        }
        let _ = allocator.free(mem_image.allocation);
    }

    let depth_buffer_sg = {
        let depth_buffer_mem_image =
            create_depth_buffer(device, &surface_extent, depth_format, allocator)?;

        guard(depth_buffer_mem_image, |mem_image| {
            log::warn!("depth buffer mem image scopeguard");
            unsafe {
                device.destroy_image(mem_image.image, None);
                device.destroy_image_view(mem_image.view, None);
            }
            let _ = allocator.free(mem_image.allocation);
        })
    };

    Ok(ResizeResult {
        surface_capabilities,
        surface_extent,
        swapchain,
        swapchain_images,
        swapchain_image_views: swapchain_image_view_sgs
            .into_iter()
            .map(|sg| ScopeGuard::into_inner(sg))
            .collect(),
        depth_buffer_mem_image: ScopeGuard::into_inner(depth_buffer_sg),
    })
}

fn check_device_suitability(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    required_extensions: &Vec<&std::ffi::CStr>,
    properties: &vk::PhysicalDeviceProperties,
) -> Result<(), String> {
    // api version
    log::info!("checking api version");
    log::info!(
        "supported api version: {}.{}.{}",
        vk::api_version_major(properties.api_version),
        vk::api_version_minor(properties.api_version),
        vk::api_version_patch(properties.api_version)
    );

    if vk::api_version_major(properties.api_version) < 1
        && vk::api_version_minor(properties.api_version) < 2
    {
        return Err(String::from(
            "the device does not support API version 1.2.0",
        ));
    }

    // features
    log::info!("checking supported features");
    let features = unsafe { instance.get_physical_device_features(physical_device) };

    // TODO pass as parameter
    if features.tessellation_shader == 0 {
        return Err(String::from(
            "the device does not support tesselation shader",
        ));
    }

    log::info!("tesselation shader supported");

    if features.fill_mode_non_solid == 0 {
        return Err(String::from(
            "the device does not support fill mode non solid",
        ));
    }

    log::info!("fill mode non solid supported");

    check_required_device_extensions(instance, physical_device, required_extensions)?;

    Ok(())
}

fn check_required_device_extensions(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    required_extensions: &Vec<&std::ffi::CStr>,
) -> Result<(), String> {
    log::info!(
        "checking required device extensions: {:?}",
        required_extensions
    );

    let supported_device_extensions =
        match unsafe { instance.enumerate_device_extension_properties(physical_device) } {
            Ok(props) => props,
            Err(_) => {
                return Err(String::from(
                    "failed to enumerate instance extension properies",
                ));
            }
        };

    let mut supported_device_extensions_set = std::collections::HashSet::new();
    for vk::ExtensionProperties { extension_name, .. } in &supported_device_extensions {
        supported_device_extensions_set
            .insert(unsafe { std::ffi::CStr::from_ptr(extension_name.as_ptr()) });
    }

    for extension_name in required_extensions {
        if !supported_device_extensions_set.contains(extension_name) {
            return Err(format!(
                "device extension {:?} is not supported",
                extension_name
            ));
        }
    }

    log::info!("all extensions are supported",);

    Ok(())
}

pub fn get_physical_device<'a>(
    instance: &ash::Instance,
    required_device_extensions: &Vec<&'a std::ffi::CStr>,
) -> Result<vk::PhysicalDevice, String> {
    log::info!("enumerating physical devices");

    let devices = match unsafe { instance.enumerate_physical_devices() } {
        Ok(devices) => devices,
        Err(_) => return Err(String::from("failed to enumerate physical devices")),
    };

    log::info!("available physical devices: ");
    for &physical_device in &devices {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_name = unsafe { std::ffi::CStr::from_ptr(properties.device_name.as_ptr()) };
        log::info!("{:?}", device_name);
    }

    for physical_device in devices {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_name = unsafe { std::ffi::CStr::from_ptr(properties.device_name.as_ptr()) };

        log::info!("checking physical device: {:?}", device_name);

        if let Err(msg) = check_device_suitability(
            instance,
            physical_device,
            required_device_extensions,
            &properties,
        ) {
            log::warn!("{:?}: {}", device_name, msg);
            continue;
        }

        log::info!("selected physical device {:?}", device_name);

        return Ok(physical_device);
    }

    Err(String::from("failed to find suitable device"))
}

pub fn create_swapchain_image_views(
    device: &ash::Device,
    swapchain_images: &Vec<vk::Image>,
    surface_format: &vk::SurfaceFormatKHR,
) -> Result<Vec<vk::ImageView>, String> {
    log::info!("creating swapchain images views");

    let mut swapchain_image_views = Vec::with_capacity(swapchain_images.len());

    for (i, &image) in swapchain_images.iter().enumerate() {
        let create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(surface_format.format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build();

        let view = unsafe {
            device.create_image_view(&create_info, None).map_err(|_| {
                for &image_view in &swapchain_image_views {
                    device.destroy_image_view(image_view, None);
                }
                format!("failed to create image view {}", i)
            })?
        };

        swapchain_image_views.push(view);
    }

    log::info!("swapchain images views created");

    Ok(swapchain_image_views)
}

pub fn create_depth_buffer(
    device: &ash::Device,
    surface_extent: &vk::Extent2D,
    depth_format: vk::Format,
    allocator: &mut gpu_allocator::vulkan::Allocator,
) -> Result<MemImage, String> {
    // image
    log::info!("creating depth buffer image");

    let extent = vk::Extent3D {
        width: surface_extent.width,
        height: surface_extent.height,
        depth: 1,
    };

    let image_sg = {
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(depth_format)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .build();

        let image = unsafe {
            device
                .create_image(&image_create_info, None)
                .map_err(|_| format!("failed to create depth buffer image"))?
        };

        scopeguard::guard(image, |image| {
            log::warn!("depth buffer image scopeguard");
            unsafe {
                device.destroy_image(image, None);
            }
        })
    };

    log::info!("depth buffer image created");

    // allocation
    log::info!("allocating depth buffer image memory");

    let allocation_sg = {
        let memory_requirements = unsafe { device.get_image_memory_requirements(*image_sg) };

        let allocation_create_desc = gpu_allocator::vulkan::AllocationCreateDesc {
            name: "depth buffer image",
            requirements: memory_requirements,
            location: gpu_allocator::MemoryLocation::GpuOnly,
            linear: false,
        };

        let allocation = allocator
            .allocate(&allocation_create_desc)
            .map_err(|_| format!("failed to allocate depth buffer image memory"))?;

        scopeguard::guard(allocation, |allocation| {
            log::warn!("depth buffer image allocation scopeguard");
            let _ = allocator.free(allocation);
        })
    };

    log::info!("depth buffer image memory allocated");

    // binding
    log::info!("binding depth buffer image memory");

    unsafe {
        device
            .bind_image_memory(*image_sg, allocation_sg.memory(), allocation_sg.offset())
            .map_err(|_| format!("failed to bind depth buffer image memory"))?
    };

    log::info!("depth buffer image memory bound");

    // view
    log::info!("creating depth buffer image view");

    let image_view_sg = {
        let view_create_info = vk::ImageViewCreateInfo::builder()
            .image(*image_sg)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(depth_format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build();

        let view = unsafe {
            device
                .create_image_view(&view_create_info, None)
                .map_err(|_| format!("failed to create depth buffer image view"))?
        };

        scopeguard::guard(view, |view| {
            log::warn!("depth buffer image view scopeguard");
            unsafe {
                device.destroy_image_view(view, None);
            }
        })
    };

    log::info!("depth buffer image view created");

    Ok(MemImage {
        image: scopeguard::ScopeGuard::into_inner(image_sg),
        view: scopeguard::ScopeGuard::into_inner(image_view_sg),
        extent,
        allocation: scopeguard::ScopeGuard::into_inner(allocation_sg),
    })
}
