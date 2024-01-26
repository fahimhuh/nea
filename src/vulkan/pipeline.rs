use std::sync::Arc;

use super::{context::Context, descriptor::DescriptorSetLayout, shader::Shader};
use ash::vk;

pub struct PipelineLayout {
    context: Arc<Context>,
    pub handle: vk::PipelineLayout,
}

impl PipelineLayout {
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
    fn drop(&mut self) {
        unsafe {
            self.context
                .device
                .destroy_pipeline_layout(self.handle, None)
        }
    }
}

pub struct ComputePipeline {
    context: Arc<Context>,
    pub handle: vk::Pipeline,
}

impl ComputePipeline {
    pub fn new(context: Arc<Context>, shader: &Shader, layout: &PipelineLayout) -> Self {
        let stage = vk::PipelineShaderStageCreateInfo::builder()
            .module(shader.handle)
            .name(&shader.entry)
            .stage(shader.stage)
            .build();

        let create_info = vk::ComputePipelineCreateInfo::builder()
            .layout(layout.handle)
            .stage(stage)
            .build();

        let handle = unsafe {
            context
                .device
                .create_compute_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .unwrap()
                .remove(0)
        };

        Self { context, handle }
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_pipeline(self.handle, None) }
    }
}

pub struct GraphicsPipeline {
    context: Arc<Context>,
    pub handle: vk::Pipeline,
}

impl GraphicsPipeline {
    pub fn new(
        context: Arc<Context>,
        vertex_shader: &Shader,
        fragment_shader: &Shader,
        layout: &PipelineLayout,
        vertex_info: vk::PipelineVertexInputStateCreateInfo,
        mut render_info: vk::PipelineRenderingCreateInfo,
    ) -> Self {
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

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1);

        let raster_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0);

        let stencil_op = vk::StencilOpState::builder()
            .fail_op(vk::StencilOp::KEEP)
            .pass_op(vk::StencilOp::KEEP)
            .compare_op(vk::CompareOp::ALWAYS)
            .build();

        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(vk::CompareOp::ALWAYS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .front(stencil_op)
            .back(stencil_op);

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

        let color_blend_info =
            vk::PipelineColorBlendStateCreateInfo::builder().attachments(&color_blend_attachments);

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

        let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

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

        Self { context, handle }
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_pipeline(self.handle, None) }
    }
}
