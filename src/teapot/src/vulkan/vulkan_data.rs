use ash::vk;
use raw_window_handle::HasRawDisplayHandle;
use vulkan_base::VulkanBase;

pub struct VulkanData {
    pub vertex_shader_module: vk::ShaderModule,
    pub tese_shader_module: vk::ShaderModule,
    pub tesc_shader_module: vk::ShaderModule,
    pub fragment_shader_module: vk::ShaderModule,
}

impl VulkanData {
    pub fn new(vulkan_base: &VulkanBase) -> Result<Self, String> {
        let vertex_sm_sg = {
            let vertex_sm = vulkan_utils::create_shader_module(
                &vulkan_base.device,
                std::path::Path::new("shaders/shader.vert.spv"),
                &vulkan_base.debug_utils_loader,
                "vertex shader",
            )?;

            scopeguard::guard(vertex_sm, |sm| {
                log::warn!("vertex shader scopeguard");
                unsafe {
                    vulkan_base.device.destroy_shader_module(sm, None);
                }
            })
        };

        let tese_sm_sg = {
            let tese_sm = vulkan_utils::create_shader_module(
                &vulkan_base.device,
                std::path::Path::new("shaders/shader.tese.spv"),
                &vulkan_base.debug_utils_loader,
                "tessellation evaluation shader",
            )?;

            scopeguard::guard(tese_sm, |sm| {
                log::warn!("tessellation evaluation shader scopeguard");
                unsafe {
                    vulkan_base.device.destroy_shader_module(sm, None);
                }
            })
        };

        let tesc_sm_sg = {
            let tesc_sm = vulkan_utils::create_shader_module(
                &vulkan_base.device,
                std::path::Path::new("shaders/shader.tesc.spv"),
                &vulkan_base.debug_utils_loader,
                "tessellation control shader",
            )?;

            scopeguard::guard(tesc_sm, |sm| {
                log::warn!("tessellation control shader scopeguard");
                unsafe {
                    vulkan_base.device.destroy_shader_module(sm, None);
                }
            })
        };

        let fragment_sm_sg = {
            let fragment_sm = vulkan_utils::create_shader_module(
                &vulkan_base.device,
                std::path::Path::new("shaders/shader.frag.spv"),
                &vulkan_base.debug_utils_loader,
                "fragment shader",
            )?;

            scopeguard::guard(fragment_sm, |sm| {
                log::warn!("fragment shader scopeguard");
                unsafe {
                    vulkan_base.device.destroy_shader_module(sm, None);
                }
            })
        };

        Ok(VulkanData {
            vertex_shader_module: scopeguard::ScopeGuard::into_inner(vertex_sm_sg),
            tese_shader_module: scopeguard::ScopeGuard::into_inner(tese_sm_sg),
            tesc_shader_module: scopeguard::ScopeGuard::into_inner(tesc_sm_sg),
            fragment_shader_module: scopeguard::ScopeGuard::into_inner(fragment_sm_sg),
        })
    }

    pub fn clean(self, vulkan_base: &VulkanBase) {
        log::info!("cleaning vulkan data");

        unsafe {
            vulkan_base
                .device
                .destroy_shader_module(self.vertex_shader_module, None);
            vulkan_base
                .device
                .destroy_shader_module(self.tese_shader_module, None);
            vulkan_base
                .device
                .destroy_shader_module(self.tesc_shader_module, None);
            vulkan_base
                .device
                .destroy_shader_module(self.fragment_shader_module, None);
        }
    }
}

pub fn vulkan_clean(
    vulkan_base: &mut Option<vulkan_base::VulkanBase>,
    vulkan_data: &mut Option<super::VulkanData>,
) {
    let vk_base = vulkan_base.take().unwrap();
    let vk_data = vulkan_data.take().unwrap();

    unsafe {
        let _ = vk_base.device.device_wait_idle();
    }

    vk_data.clean(&vk_base);
    vk_base.clean();
}

pub fn get_required_instance_extensions(
    window: &winit::window::Window,
) -> Result<Vec<&'static std::ffi::CStr>, String> {
    log::info!("getting required instance extensions");

    let mut instance_extensions =
        match ash_window::enumerate_required_extensions(window.raw_display_handle()) {
            Ok(extensions) => extensions
                .to_vec()
                .into_iter()
                .map(|name| unsafe { std::ffi::CStr::from_ptr(name) })
                .collect::<Vec<&'static std::ffi::CStr>>(),
            Err(_) => {
                return Err(String::from(
                    "failed to enumerate required instance extensions",
                ));
            }
        };

    log::info!("required instance extensions: {:?}", instance_extensions);

    instance_extensions.push(ash::extensions::ext::DebugUtils::name());

    Ok(instance_extensions)
}
