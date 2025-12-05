use ash::vk;
use raw_window_handle::HasRawDisplayHandle;

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
        .cull_mode(vk::CullModeFlags::NONE)
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

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
        .build();

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
        .depth_stencil_state(&depth_stencil_state)
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
    depth_format: vk::Format,
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

    attachment_descriptions.push(
        vk::AttachmentDescription::builder()
            .format(depth_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build(),
    );

    let col_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();

    let depth_attachment_ref = vk::AttachmentReference::builder()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .build();

    let references = [col_attachment_ref];

    let mut subpass_descriptions = Vec::new();

    subpass_descriptions.push(
        vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&references)
            .depth_stencil_attachment(&depth_attachment_ref)
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

pub fn create_framebuffers(
    device: &ash::Device,
    swapchain_image_views: &Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    framebuffer_extent: vk::Extent2D,
    depth_buffer_view: vk::ImageView,
    debug_utils_loader: &ash::extensions::ext::DebugUtils,
) -> Result<Vec<vk::Framebuffer>, String> {
    let mut framebuffers = Vec::with_capacity(swapchain_image_views.len());

    for (i, &view) in swapchain_image_views.iter().enumerate() {
        let attachments = [view, depth_buffer_view];

        let create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(framebuffer_extent.width)
            .height(framebuffer_extent.height)
            .layers(1)
            .build();

        let framebuffer = unsafe {
            device.create_framebuffer(&create_info, None).map_err(|_| {
                for &fb in &framebuffers {
                    device.destroy_framebuffer(fb, None);
                }
                format!("failed to create framebuffer {}", i)
            })?
        };

        framebuffers.push(framebuffer);

        vulkan_utils::set_debug_utils_object_name2(
            debug_utils_loader,
            device.handle(),
            framebuffer,
            &format!("framebuffer {}", i),
        );
    }

    Ok(framebuffers)
}

pub fn create_command_pools(
    device: &ash::Device,
    queue_family: u32,
    debug_utils_loader: &ash::extensions::ext::DebugUtils,
) -> Result<Vec<vk::CommandPool>, String> {
    log::info!("creating command pools");

    let create_info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::TRANSIENT)
        .queue_family_index(queue_family);

    let mut command_pools = Vec::with_capacity(crate::CONCURRENT_RESOURCE_COUNT as usize);

    for i in 0..crate::CONCURRENT_RESOURCE_COUNT {
        let command_pool = unsafe {
            device
                .create_command_pool(&create_info, None)
                .map_err(|_| {
                    for &cp in &command_pools {
                        device.destroy_command_pool(cp, None);
                    }

                    format!("failed to create command pool {}", i)
                })?
        };

        command_pools.push(command_pool);

        vulkan_utils::set_debug_utils_object_name2(
            debug_utils_loader,
            device.handle(),
            command_pool,
            &format!("command pool {}", i),
        );
    }

    log::info!("command pools created");

    Ok(command_pools)
}

pub fn create_descriptor_pools(
    device: &ash::Device,
    debug_utils_loader: &ash::extensions::ext::DebugUtils,
) -> Result<Vec<vk::DescriptorPool>, String> {
    log::info!("creating descriptor pools");

    let pool_size_1 = vk::DescriptorPoolSize {
        ty: vk::DescriptorType::STORAGE_BUFFER,
        descriptor_count: 100,
    };

    let pool_size_2 = vk::DescriptorPoolSize {
        ty: vk::DescriptorType::UNIFORM_BUFFER,
        descriptor_count: 100,
    };

    let sizes = [pool_size_1, pool_size_2];
    let create_info = vk::DescriptorPoolCreateInfo::builder()
        .max_sets(100)
        .pool_sizes(&sizes)
        .build();

    let mut descriptor_pools = Vec::with_capacity(crate::CONCURRENT_RESOURCE_COUNT as usize);

    for i in 0..crate::CONCURRENT_RESOURCE_COUNT {
        let pool = unsafe {
            device
                .create_descriptor_pool(&create_info, None)
                .map_err(|_| {
                    for &p in &descriptor_pools {
                        device.destroy_descriptor_pool(p, None);
                    }
                    format!("failed to create descriptor pool {}", i)
                })?
        };

        vulkan_utils::set_debug_utils_object_name2(
            debug_utils_loader,
            device.handle(),
            pool,
            &format!("descriptor pool {}", i),
        );

        descriptor_pools.push(pool);
    }

    log::info!("descriptor pools created");

    Ok(descriptor_pools)
}

pub fn create_fences(
    device: &ash::Device,
    debug_utils_loader: &ash::extensions::ext::DebugUtils,
) -> Result<Vec<vk::Fence>, String> {
    log::info!("creating fences");

    let create_info = vk::FenceCreateInfo::builder()
        .flags(vk::FenceCreateFlags::SIGNALED)
        .build();

    let mut fences = Vec::with_capacity(crate::CONCURRENT_RESOURCE_COUNT as usize);

    for i in 0..crate::CONCURRENT_RESOURCE_COUNT {
        let fence = unsafe {
            device.create_fence(&create_info, None).map_err(|_| {
                for &f in &fences {
                    device.destroy_fence(f, None);
                }

                format!("failed to create fence {}", i)
            })?
        };

        fences.push(fence);

        vulkan_utils::set_debug_utils_object_name2(
            debug_utils_loader,
            device.handle(),
            fence,
            &format!("fence {}", i),
        );
    }

    log::info!("fences created");

    Ok(fences)
}
