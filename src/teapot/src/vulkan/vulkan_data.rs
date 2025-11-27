use crate::teapot_data;
use crate::vulkan;
use ash::vk;
use raw_window_handle::HasRawDisplayHandle;
use scopeguard::{guard, ScopeGuard};
use std::cell::RefCell;
use vulkan_base::VulkanBase;

pub struct VulkanData {
    pub vertex_shader_module: vk::ShaderModule,
    pub tese_shader_module: vk::ShaderModule,
    pub tesc_shader_module: vk::ShaderModule,
    pub fragment_shader_module: vk::ShaderModule,
    pub control_points_mem_buffer: vulkan_utils::MemBuffer,
    pub patches_mem_buffer: vulkan_utils::MemBuffer,
    pub patch_point_count: u32,
    pub instances_mem_buffer: vulkan_utils::MemBuffer,
    pub uniform_mem_buffers: Vec<vulkan_utils::MemBuffer>,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub pipeline_layout: vk::PipelineLayout,
    pub render_pass: vk::RenderPass,
    pub solid_pipeline: vk::Pipeline,
    pub wireframe_pipeline: vk::Pipeline,
}

impl VulkanData {
    pub fn new(vulkan_base: &mut VulkanBase) -> Result<Self, String> {
        let device = &vulkan_base.device;
        let allocator_rc = RefCell::new(&mut vulkan_base.allocator);

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

        let teapot_data = teapot_data::TeapotData::new();

        let control_points_mem_buffer_sg = {
            let control_points_mem_buffer = vulkan_utils::create_gpu_buffer_init(
                &vulkan_base.device,
                *allocator_rc.borrow_mut(),
                &vulkan_base.debug_utils_loader,
                vulkan_base.queue_family,
                vulkan_base.queue,
                teapot_data.get_control_points_slice(),
                vk::BufferUsageFlags::STORAGE_BUFFER,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::VERTEX_SHADER,
                "control points buffer",
            )?;

            guard(control_points_mem_buffer, |mem_buffer| {
                log::warn!("control points buffer scopeguard");
                unsafe {
                    device.destroy_buffer(mem_buffer.buffer, None);
                }
                let _ = allocator_rc.borrow_mut().free(mem_buffer.allocation);
            })
        };

        let patches_mem_buffer_sg = {
            let patches_mem_buffer = vulkan_utils::create_gpu_buffer_init(
                &vulkan_base.device,
                *allocator_rc.borrow_mut(),
                &vulkan_base.debug_utils_loader,
                vulkan_base.queue_family,
                vulkan_base.queue,
                teapot_data.get_patches_slice(),
                vk::BufferUsageFlags::INDEX_BUFFER,
                vk::AccessFlags::INDEX_READ,
                vk::PipelineStageFlags::VERTEX_INPUT,
                "patches buffer",
            )?;

            guard(patches_mem_buffer, |mem_buffer| {
                log::warn!("patches buffer scopeguard");
                unsafe {
                    device.destroy_buffer(mem_buffer.buffer, None);
                }
                let _ = allocator_rc.borrow_mut().free(mem_buffer.allocation);
            })
        };

        let patch_point_count = teapot_data.get_patch_point_count();

        let instances_mem_buffer_sg = {
            let instances_mem_buffer = vulkan_utils::create_gpu_buffer_init(
                &vulkan_base.device,
                *allocator_rc.borrow_mut(),
                &vulkan_base.debug_utils_loader,
                vulkan_base.queue_family,
                vulkan_base.queue,
                teapot_data.get_instances_slice(),
                vk::BufferUsageFlags::STORAGE_BUFFER,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER,
                "instances buffer",
            )?;

            guard(instances_mem_buffer, |mem_buffer| {
                log::warn!("instances buffer scopeguard");
                unsafe {
                    device.destroy_buffer(mem_buffer.buffer, None);
                }
                let _ = allocator_rc.borrow_mut().free(mem_buffer.allocation);
            })
        };

        let uniform_mem_buffers_sg = {
            let mut mem_buffers = Vec::with_capacity(crate::CONCURRENT_RESOURCE_COUNT as usize);
            for i in 0..crate::CONCURRENT_RESOURCE_COUNT {
                let mem_buffer = vulkan_utils::create_buffer(
                    &vulkan_base.device,
                    *allocator_rc.borrow_mut(),
                    &vulkan_base.debug_utils_loader,
                    (16 * std::mem::size_of::<f32>()) as vk::DeviceSize,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                    gpu_allocator::MemoryLocation::CpuToGpu,
                    &format!("uniform buffer {}", i),
                )?;

                mem_buffers.push(mem_buffer);
            }

            guard(mem_buffers, |mem_buffers| {
                log::warn!("uniform buffers scopeguard");

                for mem_buffer in mem_buffers {
                    unsafe {
                        device.destroy_buffer(mem_buffer.buffer, None);
                    }
                    let _ = allocator_rc.borrow_mut().free(mem_buffer.allocation);
                }
            })
        };

        let descriptor_set_layout_sg = {
            let descriptor_set_layout = vulkan::create_descriptor_set_layout(
                &vulkan_base.device,
                &vulkan_base.debug_utils_loader,
            )?;

            guard(descriptor_set_layout, |layout| {
                log::warn!("descriptor set layout scopeguard");
                unsafe {
                    device.destroy_descriptor_set_layout(layout, None);
                }
            })
        };

        let pipeline_layout_sg = {
            let pipeline_layout = vulkan::create_pipeline_layout(
                &vulkan_base.device,
                *descriptor_set_layout_sg,
                &vulkan_base.debug_utils_loader,
            )?;

            guard(pipeline_layout, |layout| {
                log::warn!("pipeline layout scopeguard");
                unsafe {
                    device.destroy_pipeline_layout(layout, None);
                }
            })
        };

        let render_pass_sg = {
            let render_pass = vulkan::create_render_pass(
                &vulkan_base.device,
                vulkan_base.surface_format.format,
                &vulkan_base.debug_utils_loader,
            )?;

            guard(render_pass, |render_pass| {
                log::warn!("render pass scopeguard");
                unsafe {
                    device.destroy_render_pass(render_pass, None);
                }
            })
        };

        let (solid_pipeline_sg, wireframe_pipeline_sg) = {
            let (solid_pipeline, wireframe_pipeline) = vulkan::create_pipelines(
                &vulkan_base.device,
                *vertex_sm_sg,
                *tesc_sm_sg,
                *tese_sm_sg,
                *fragment_sm_sg,
                *pipeline_layout_sg,
                *render_pass_sg,
                &vulkan_base.debug_utils_loader,
            )?;

            let sg_1 = guard(solid_pipeline, |pipeline| {
                log::warn!("solid pipeline scopeguard");
                unsafe {
                    device.destroy_pipeline(pipeline, None);
                }
            });

            let sg_2 = guard(wireframe_pipeline, |pipeline| {
                log::warn!("wireframe pipeline scopeguard");
                unsafe {
                    device.destroy_pipeline(pipeline, None);
                }
            });

            (sg_1, sg_2)
        };

        Ok(VulkanData {
            vertex_shader_module: ScopeGuard::into_inner(vertex_sm_sg),
            tese_shader_module: ScopeGuard::into_inner(tese_sm_sg),
            tesc_shader_module: ScopeGuard::into_inner(tesc_sm_sg),
            fragment_shader_module: ScopeGuard::into_inner(fragment_sm_sg),
            control_points_mem_buffer: ScopeGuard::into_inner(control_points_mem_buffer_sg),
            patches_mem_buffer: ScopeGuard::into_inner(patches_mem_buffer_sg),
            patch_point_count,
            instances_mem_buffer: ScopeGuard::into_inner(instances_mem_buffer_sg),
            uniform_mem_buffers: ScopeGuard::into_inner(uniform_mem_buffers_sg),
            descriptor_set_layout: ScopeGuard::into_inner(descriptor_set_layout_sg),
            pipeline_layout: ScopeGuard::into_inner(pipeline_layout_sg),
            render_pass: ScopeGuard::into_inner(render_pass_sg),
            solid_pipeline: ScopeGuard::into_inner(solid_pipeline_sg),
            wireframe_pipeline: ScopeGuard::into_inner(wireframe_pipeline_sg),
        })
    }

    pub fn clean(self, vulkan_base: &mut VulkanBase) {
        log::info!("cleaning vulkan data");

        unsafe {
            let device = &vulkan_base.device;
            let allocator = &mut vulkan_base.allocator;

            device.destroy_shader_module(self.vertex_shader_module, None);
            device.destroy_shader_module(self.tese_shader_module, None);
            device.destroy_shader_module(self.tesc_shader_module, None);
            device.destroy_shader_module(self.fragment_shader_module, None);

            device.destroy_buffer(self.control_points_mem_buffer.buffer, None);
            let _ = allocator.free(self.control_points_mem_buffer.allocation);

            device.destroy_buffer(self.patches_mem_buffer.buffer, None);
            let _ = allocator.free(self.patches_mem_buffer.allocation);

            device.destroy_buffer(self.instances_mem_buffer.buffer, None);
            let _ = allocator.free(self.instances_mem_buffer.allocation);

            for mem_buffer in self.uniform_mem_buffers {
                device.destroy_buffer(mem_buffer.buffer, None);
                let _ = allocator.free(mem_buffer.allocation);
            }

            vulkan_base
                .device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            vulkan_base
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);

            vulkan_base
                .device
                .destroy_render_pass(self.render_pass, None);

            vulkan_base
                .device
                .destroy_pipeline(self.solid_pipeline, None);

            vulkan_base
                .device
                .destroy_pipeline(self.wireframe_pipeline, None);
        }
    }
}

pub fn vulkan_clean(
    vulkan_base: &mut Option<vulkan_base::VulkanBase>,
    vulkan_data: &mut Option<super::VulkanData>,
) {
    let mut vk_base = vulkan_base.take().unwrap();
    let vk_data = vulkan_data.take().unwrap();

    unsafe {
        let _ = vk_base.device.device_wait_idle();
    }

    vk_data.clean(&mut vk_base);
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

pub fn create_descriptor_set_layout(
    device: &ash::Device,
    debug_utils_loader: &ash::extensions::ext::DebugUtils,
) -> Result<vk::DescriptorSetLayout, String> {
    log::info!("creating descriptor set layout");

    let control_points_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .build();

    let patch_data_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(1)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::TESSELLATION_EVALUATION)
        .build();

    let uniform_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(2)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::TESSELLATION_EVALUATION)
        .build();

    let bindings = [control_points_binding, patch_data_binding, uniform_binding];
    let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
        .bindings(&bindings)
        .build();

    let descriptor_set_layout = unsafe {
        device
            .create_descriptor_set_layout(&create_info, None)
            .map_err(|_| String::from("failed to create descriptor set layout"))?
    };

    vulkan_utils::set_debug_utils_object_name2(
        debug_utils_loader,
        device.handle(),
        descriptor_set_layout,
        "descriptor set layout",
    );

    log::info!("descriptor set layout created");

    Ok(descriptor_set_layout)
}

pub fn create_pipeline_layout(
    device: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
    debug_utils_loader: &ash::extensions::ext::DebugUtils,
) -> Result<vk::PipelineLayout, String> {
    log::info!("creating pipeline layout");

    let push_const_range = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::TESSELLATION_CONTROL,
        offset: 0,
        size: 4,
    };

    let layouts = [descriptor_set_layout];
    let ranges = [push_const_range];
    let create_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(&layouts)
        .push_constant_ranges(&ranges)
        .build();

    let pipeline_layout = unsafe {
        device
            .create_pipeline_layout(&create_info, None)
            .map_err(|_| String::from("failed to create pipeline layout"))?
    };

    vulkan_utils::set_debug_utils_object_name2(
        debug_utils_loader,
        device.handle(),
        pipeline_layout,
        "pipeline layout",
    );

    log::info!("pipeline layout created");

    Ok(pipeline_layout)
}

pub fn create_pipelines(
    device: &ash::Device,
    vertex_shader_module: vk::ShaderModule,
    tess_control_shader_module: vk::ShaderModule,
    tess_eval_shader_module: vk::ShaderModule,
    fragment_shader_module: vk::ShaderModule,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    debug_utils_loader: &ash::extensions::ext::DebugUtils,
) -> Result<(vk::Pipeline, vk::Pipeline), String> {
    log::info!("creating pipelines");

    let shader_entry_name = std::ffi::CString::new("main").unwrap();

    let vs_state = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_shader_module)
        .name(&shader_entry_name)
        .build();

    let tc_state = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::TESSELLATION_CONTROL)
        .module(tess_control_shader_module)
        .name(&shader_entry_name)
        .build();

    let te_state = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::TESSELLATION_EVALUATION)
        .module(tess_eval_shader_module)
        .name(&shader_entry_name)
        .build();

    let fs_state = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(fragment_shader_module)
        .name(&shader_entry_name)
        .build();

    let ia_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::PATCH_LIST)
        .build();

    let raster_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE)
        .line_width(1.0f32)
        .build();

    let col_blend_attachment_state = vk::PipelineColorBlendAttachmentState::builder()
        .blend_enable(false)
        .color_write_mask(
            vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        )
        .build();

    let attachments = [col_blend_attachment_state];
    let col_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .attachments(&attachments)
        .build();

    let states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dyn_state = vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(&states)
        .build();

    let viewports = [vk::Viewport {
        ..Default::default()
    }];
    let scissors = [vk::Rect2D {
        ..Default::default()
    }];

    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(&viewports)
        .scissors(&scissors)
        .build();

    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let tessellation_state = vk::PipelineTessellationStateCreateInfo::builder()
        .patch_control_points(16)
        .build();

    let stages = [vs_state, tc_state, te_state, fs_state];

    let vert_inp_state = vk::PipelineVertexInputStateCreateInfo::builder().build();

    let solid_pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
        .flags(vk::PipelineCreateFlags::ALLOW_DERIVATIVES)
        .stages(&stages)
        .input_assembly_state(&ia_state)
        .rasterization_state(&raster_state)
        .color_blend_state(&col_blend_state)
        .dynamic_state(&dyn_state)
        .viewport_state(&viewport_state)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)
        .multisample_state(&multisample_state)
        .tessellation_state(&tessellation_state)
        .vertex_input_state(&vert_inp_state)
        .build();

    let raster_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .polygon_mode(vk::PolygonMode::LINE)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::CLOCKWISE)
        .line_width(1.0f32)
        .build();

    let mut wireframe_pipeline_create_info = solid_pipeline_create_info;
    wireframe_pipeline_create_info.flags = vk::PipelineCreateFlags::DERIVATIVE;
    wireframe_pipeline_create_info.p_rasterization_state = &raster_state;
    wireframe_pipeline_create_info.base_pipeline_index = 0;

    let pipelines = unsafe {
        device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[solid_pipeline_create_info, wireframe_pipeline_create_info],
                None,
            )
            .map_err(|_| String::from("failed to create pipelines"))?
    };

    let solid_pipeline = pipelines[0];
    let wireframe_pipeline = pipelines[1];

    vulkan_utils::set_debug_utils_object_name2(
        debug_utils_loader,
        device.handle(),
        solid_pipeline,
        "solid pipeline",
    );

    vulkan_utils::set_debug_utils_object_name2(
        debug_utils_loader,
        device.handle(),
        wireframe_pipeline,
        "wireframe pipeline",
    );

    log::info!("pipelines created");

    Ok((solid_pipeline, wireframe_pipeline))
}

pub fn create_render_pass(
    device: &ash::Device,
    surface_format: vk::Format,
    debug_utils_loader: &ash::extensions::ext::DebugUtils,
) -> Result<vk::RenderPass, String> {
    log::info!("creating render pass");

    let mut attachment_descriptions = Vec::new();

    attachment_descriptions.push(
        vk::AttachmentDescription::builder()
            .format(surface_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build(),
    );

    let col_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();

    let references = [col_attachment_ref];

    let mut subpass_descriptions = Vec::new();

    subpass_descriptions.push(
        vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&references)
            .build(),
    );

    let create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachment_descriptions)
        .subpasses(&subpass_descriptions);

    let render_pass = unsafe {
        device
            .create_render_pass(&create_info, None)
            .map_err(|_| String::from("failed to create render pass"))?
    };

    vulkan_utils::set_debug_utils_object_name2(
        debug_utils_loader,
        device.handle(),
        render_pass,
        "render pass",
    );

    log::info!("render pass created");

    Ok(render_pass)
}
