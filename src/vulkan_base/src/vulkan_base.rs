use ash::extensions::ext;
use ash::extensions::khr;
use ash::vk;
use gpu_allocator::vulkan;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

pub fn create_entry() -> ash::Entry {
    log::info!("creating entry");

    let entry = ash::Entry::linked();

    log::info!("entry created");

    entry
}

pub fn check_instance_version(entry: &ash::Entry) -> Result<(), String> {
    log::info!("checking instance version");

    let api_version = match entry.try_enumerate_instance_version() {
        Ok(result) => match result {
            Some(version) => version,
            None => vk::make_api_version(0, 1, 0, 0),
        },
        Err(_) => {
            return Err(String::from("failed to enumerate instance version"));
        }
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

    Ok(())
}

pub fn check_required_instance_extensions<'a>(
    entry: &ash::Entry,
    required_instance_extensions: &Vec<&'a std::ffi::CStr>,
) -> Result<(), String> {
    log::info!(
        "checking required instance extensions: {:?}",
        required_instance_extensions
    );

    let supported_instance_extensions = match entry.enumerate_instance_extension_properties(None) {
        Ok(props) => props,
        Err(_) => {
            return Err(String::from(
                "failed to enumerate instance extension properties",
            ));
        }
    };

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

    log::info!("all extensions are supported",);

    Ok(())
}

pub fn create_instance<'a>(
    entry: &ash::Entry,
    instance_extensions: &Vec<&'a std::ffi::CStr>,
) -> Result<ash::Instance, String> {
    log::info!("creating instance");

    let extension_names_raw = instance_extensions
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

    log::info!("instance created");

    Ok(instance)
}

pub fn create_debug_utils_loader(entry: &ash::Entry, instance: &ash::Instance) -> ext::DebugUtils {
    let debug_utils_loader = ext::DebugUtils::new(&entry, &instance);

    log::info!("debug utils loader created");

    debug_utils_loader
}

pub fn create_surface_loader(entry: &ash::Entry, instance: &ash::Instance) -> khr::Surface {
    let surface_loader = khr::Surface::new(&entry, &instance);

    log::info!("surface loader created");

    surface_loader
}

pub fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &winit::window::Window,
) -> Result<vk::SurfaceKHR, String> {
    log::info!("creating surface");

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

    log::info!("surface created");

    Ok(surface)
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

pub fn get_physical_device_properties(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> vk::PhysicalDeviceProperties {
    unsafe { instance.get_physical_device_properties(physical_device) }
}

pub fn get_surface_format(
    physical_device: vk::PhysicalDevice,
    surface_loader: &khr::Surface,
    surface: vk::SurfaceKHR,
) -> Result<vk::SurfaceFormatKHR, String> {
    log::info!("getting surface format");

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

    for f in &formats {
        if f.format == vk::Format::B8G8R8A8_UNORM
            && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            let surface_format = vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            };

            log::info!("selected surface format: {:?}", surface_format);

            return Ok(surface_format);
        }
    }

    log::info!("selected first surface format: {:?}", formats[0]);

    Ok(formats[0])
}

pub fn get_present_mode(
    physical_device: vk::PhysicalDevice,
    surface_loader: &khr::Surface,
    surface: vk::SurfaceKHR,
) -> Result<vk::PresentModeKHR, String> {
    log::info!("getting present mode");

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

    if modes.contains(&vk::PresentModeKHR::MAILBOX) {
        let present_mode = vk::PresentModeKHR::MAILBOX;

        log::info!("selected present mode: {:?}", present_mode);

        return Ok(present_mode);
    }

    if modes.contains(&vk::PresentModeKHR::IMMEDIATE) {
        let present_mode = vk::PresentModeKHR::IMMEDIATE;

        log::info!("selected present mode: {:?}", present_mode);

        return Ok(present_mode);
    }

    let present_mode = vk::PresentModeKHR::FIFO;

    log::info!("selected present mode: {:?}", present_mode);

    Ok(present_mode)
}

pub fn get_queue_family(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    surface_loader: &khr::Surface,
    surface: vk::SurfaceKHR,
) -> Result<u32, String> {
    log::info!("getting queue family");

    let props = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

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
                log::info!("selected queue family: {}", ind);
                return Ok(ind as u32);
            }
        }
    }

    Err(String::from(
        "failed to find graphics queue with present support",
    ))
}

pub fn get_depth_format(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<vk::Format, String> {
    log::info!("getting depth format");

    let format_candidates = [
        vk::Format::D16_UNORM_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
        vk::Format::D32_SFLOAT_S8_UINT,
    ];

    for &format in &format_candidates {
        let props =
            unsafe { instance.get_physical_device_format_properties(physical_device, format) };

        if props
            .optimal_tiling_features
            .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
        {
            log::info!("selected depth format: {:?}", format);
            return Ok(format);
        }
    }

    Err(String::from("failed to find depth format"))
}

pub fn create_logical_device<'a>(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_family: u32,
    device_extensions: &Vec<&'a std::ffi::CStr>,
) -> Result<ash::Device, String> {
    log::info!("creating logical devices");

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

    // TODO pass features parameter
    let features = vk::PhysicalDeviceFeatures::builder()
        .tessellation_shader(true)
        .fill_mode_non_solid(true)
        .build();

    let device_extensions_raw = device_extensions
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

    log::info!("logical device created");

    return Ok(device);
}

pub fn get_queue(device: &ash::Device, queue_family: u32) -> vk::Queue {
    let queue = unsafe { device.get_device_queue(queue_family, 0) };

    log::info!("queue got");

    queue
}

pub fn create_allocator(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
) -> Result<vulkan::Allocator, String> {
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

    let allocator = vulkan::Allocator::new(&create_info)
        .map_err(|_| String::from("failed to create allocator"))?;

    log::info!("allocator created");

    Ok(allocator)
}
