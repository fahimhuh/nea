use super::{context::Context, descriptor::DescriptorSetLayout, shader::Shader};
use ash::vk;
use std::sync::Arc;

// A pipeline layout is a collection of descriptor sets and push constants that define the layout of a pipeline.
// It defines the resources that are accessible to the shaders in the pipeline.
pub struct PipelineLayout {
    // The context is a handle to the Vulkan instance, device, and queue.
    context: Arc<Context>,
    // The handle is a handle to the Vulkan pipeline layout object.
    pub handle: vk::PipelineLayout,
}

impl PipelineLayout {
    // Creates a new pipeline layout with the specified context, push constants, and descriptor sets.
    pub fn new(
        context: Arc<Context>,
        push_constants: vk::PushConstantRange,
        descriptor_sets: &[DescriptorSetLayout],
    ) -> Self {
        let sets = descriptor_sets
            .into_iter()
            .map(|d| d.handle)
            .collect::<Vec<_>>();

        let create_info = vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(std::slice::from_ref(&push_constants))
            .set_layouts(&sets);

        let handle = unsafe {
            context
                .device
                .create_pipeline_layout(&create_info, None)
                .unwrap()
        };

        Self { context, handle }
    }
}

impl Drop for PipelineLayout {
    // Destroys the pipeline layout and frees its resources.
    fn drop(&mut self) {
        unsafe {
            self.context
                .device
                .destroy_pipeline_layout(self.handle, None)
        }
    }
}

// A compute pipeline contains the state and GPU configuration for executing a compute shader program.
pub struct ComputePipeline {
    // The context is a handle to the Vulkan instance, device, and queue.
    context: Arc<Context>,
    // The handle is a handle to the Vulkan compute pipeline object.
    pub handle: vk::Pipeline,
}

impl ComputePipeline {
    // Creates a new compute pipeline with the specified context, shader, and pipeline layout.
    pub fn new(context: Arc<Context>, shader: &Shader, layout: &PipelineLayout) -> Self {
        // Create the shader stage using the Vulkan shader module.
        let stage = vk::PipelineShaderStageCreateInfo::builder()
            .module(shader.handle)
            .name(&shader.entry)
            .stage(shader.stage)
            .build();

        // Create the compute pipeline using the Vulkan device.
        let create_info = vk::ComputePipelineCreateInfo::builder()
            .layout(layout.handle)
            .stage(stage)
            .build();

        // Return the new compute pipeline.
        let handle = unsafe {
            context
                .device
                .create_compute_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .unwrap()
                .remove(0)
        };

        // Return the new compute pipeline.
        Self { context, handle }
    }
}

impl Drop for ComputePipeline {
    // Destroys the compute pipeline and frees its resources.
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_pipeline(self.handle, None) }
    }
}

// A graphics pipeline contains the state and GPU configuration for executing a graphics shader program.
// using the rasterizer. It has two programmable stages: the vertex shader and the fragment shader.
// It also has fixed-function stages for input assembly, vertex input, viewport transformation, rasterization,
// fragment shading, color blending, and multisampling.. quite complex!
pub struct GraphicsPipeline {
    // The context is a handle to the Vulkan instance, device, and queue.
    context: Arc<Context>,
    // The handle is a handle to the Vulkan graphics pipeline object.
    pub handle: vk::Pipeline,
}

impl GraphicsPipeline {
    // Creates a new graphics pipeline with the specified context, shaders, pipeline layout, vertex input state, and render pass.
    pub fn new(
        context: Arc<Context>,
        vertex_shader: &Shader,
        fragment_shader: &Shader,
        layout: &PipelineLayout,
        vertex_info: vk::PipelineVertexInputStateCreateInfo,
        mut render_info: vk::PipelineRenderingCreateInfo,
    ) -> Self {
        // Create the shader stages using the Vulkan shader modules.
        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex_shader.handle)
                .name(&vertex_shader.entry)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_shader.handle)
                .name(&fragment_shader.entry)
                .build(),
        ];

        // Hardcode the input assembly to use triangle lists.
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        // Hardcode the viewport and scissor to use the entire framebuffer.
        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1);

        // Hardcode the rasterization to use counter-clockwise winding order and no culling.
        let raster_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0);

        // Hardcode the stencil test to always pass.
        let stencil_op = vk::StencilOpState::builder()
            .fail_op(vk::StencilOp::KEEP)
            .pass_op(vk::StencilOp::KEEP)
            .compare_op(vk::CompareOp::ALWAYS)
            .build();

        // Hardcode the depth test to always pass.
        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(vk::CompareOp::ALWAYS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .front(stencil_op)
            .back(stencil_op);

        // Hardcode the color blend to use alpha blending.
        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .build()];

        // Hardcode the color blend to use alpha blending.
        let color_blend_info =
            vk::PipelineColorBlendStateCreateInfo::builder().attachments(&color_blend_attachments);

        // Hardcode the dynamic state to use viewport and scissor.
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

        // Hardcode the multisample to use 1 sample per pixel (effectively disabling anti-aliasing).
        let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Create the graphics pipeline using the Vulkan devices
        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_info)
            .rasterization_state(&raster_info)
            .multisample_state(&multisample_info)
            .depth_stencil_state(&depth_stencil_info)
            .color_blend_state(&color_blend_info)
            .dynamic_state(&dynamic_state_info)
            .layout(layout.handle)
            .push_next(&mut render_info);

        let handle = unsafe {
            context.device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&pipeline_create_info),
                None,
            )
        }
        .unwrap()
        .remove(0);

        // Return the new graphics pipeline.
        Self { context, handle }
    }
}

impl Drop for GraphicsPipeline {
    // Destroys the graphics pipeline and frees its resources.
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_pipeline(self.handle, None) }
    }
}
