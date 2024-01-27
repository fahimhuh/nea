use super::{buffer::Buffer, context::Context, image::Image, pipeline::{ComputePipeline, GraphicsPipeline, PipelineLayout}};
use ash::vk;
use std::sync::Arc;

pub struct CommandPool {
    context: Arc<Context>,
    pub handle: vk::CommandPool,
}

impl CommandPool {
    pub fn new(context: Arc<Context>, queue_family: u32) -> Self {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family)
            .flags(vk::CommandPoolCreateFlags::TRANSIENT);
        let handle = unsafe {
            context
                .device
                .create_command_pool(&create_info, None)
                .unwrap()
        };
        Self { context, handle }
    }

    pub fn reset(&self) {
        unsafe {
            self.context
                .device
                .reset_command_pool(self.handle, vk::CommandPoolResetFlags::empty())
                .unwrap();
        }
    }

    pub fn allocate(&self) -> CommandList {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.handle);

        let handle = unsafe { self.context.device.allocate_command_buffers(&allocate_info) }
            .unwrap()
            .remove(0);

        CommandList {
            context: self.context.clone(),
            handle,
        }
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_command_pool(self.handle, None) }
    }
}

pub struct CommandList {
    pub context: Arc<Context>,
    pub handle: vk::CommandBuffer,
}

impl CommandList {
    pub fn begin(&self) {
        unsafe {
            self.context
                .device
                .begin_command_buffer(self.handle, &vk::CommandBufferBeginInfo::default())
                .unwrap()
        }
    }

    pub fn end(&self) {
        unsafe { self.context.device.end_command_buffer(self.handle).unwrap() };
    }

    pub fn bind_compute_pipeline(&self, pipeline: &ComputePipeline) {
        unsafe {
            self.context.device.cmd_bind_pipeline(
                self.handle,
                vk::PipelineBindPoint::COMPUTE,
                pipeline.handle,
            )
        };
    }

    pub fn dispatch(&self, x: u32, y: u32, z: u32) {
        unsafe { self.context.device.cmd_dispatch(self.handle, x, y, z) };
    }

    pub fn pipeline_barrier(
        &self,
        image_memory_barriers: &[vk::ImageMemoryBarrier2],
        buffer_barriers: &[vk::BufferMemoryBarrier2],
    ) {
        let dependency_info = vk::DependencyInfo::builder()
            .image_memory_barriers(image_memory_barriers)
            .buffer_memory_barriers(buffer_barriers);
        unsafe {
            self.context
                .device
                .cmd_pipeline_barrier2(self.handle, &dependency_info)
        };
    }

    pub fn push_constants<T: bytemuck::NoUninit>(
        &self,
        layout: &PipelineLayout,
        stage: vk::ShaderStageFlags,
        data: T,
    ) {
        unsafe {
            self.context.device.cmd_push_constants(
                self.handle,
                layout.handle,
                stage,
                0,
                bytemuck::bytes_of(&data),
            )
        };
    }

    pub fn bind_descriptor_sets(
        &self,
        bind_point: vk::PipelineBindPoint,
        layout: &PipelineLayout,
        sets: &[vk::DescriptorSet],
    ) {
        unsafe {
            self.context.device.cmd_bind_descriptor_sets(
                self.handle,
                bind_point,
                layout.handle,
                0,
                sets,
                &[],
            );
        }
    }

    pub fn copy_to_image(&self, buffer: &Buffer, image: &Image, region: &[vk::BufferImageCopy]) {
        unsafe {
            self.context.device.cmd_copy_buffer_to_image(
                self.handle,
                buffer.handle,
                image.handle,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                region,
            );
        }
    }

    pub fn blit(&self, src: &Image, dst: &Image, regions: &[vk::ImageBlit]) {
        unsafe {
            self.context.device.cmd_blit_image(
                self.handle,
                src.handle,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                dst.handle,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                regions,
                vk::Filter::NEAREST,
            );
        }
    }

    pub fn begin_rendering(&self, begin_info: vk::RenderingInfo) {
        unsafe {
            self.context
                .device
                .cmd_begin_rendering(self.handle, &begin_info)
        }
    }

    pub fn end_rendering(&self) {
        unsafe {
            self.context
                .device
                .cmd_end_rendering(self.handle)
        }
    }

    pub fn bind_graphics_pipeline(&self, pipeline: &GraphicsPipeline) {
        unsafe {
            self.context.device.cmd_bind_pipeline(
                self.handle,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.handle,
            )
        };
    }

    pub fn bind_vertex_buffer(&self, buffer: &Buffer) {
        unsafe {
            self.context
                .device
                .cmd_bind_vertex_buffers(self.handle, 0, &[buffer.handle], &[0]);
        }
    }

    pub fn bind_index_buffer(&self, buffer: &Buffer) {
        unsafe {
            self.context.device.cmd_bind_index_buffer(
                self.handle,
                buffer.handle,
                0,
                vk::IndexType::UINT32,
            );
        }
    }

    pub fn set_viewport(&self, x: f32, y: f32, width: f32, height: f32) {
        unsafe {
            self.context.device.cmd_set_viewport(
                self.handle,
                0,
                &[vk::Viewport {
                    x,
                    y,
                    width,
                    height,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
        }
    }

    pub fn set_scissor(&self, offset: vk::Offset2D, extent: vk::Extent2D) {
        unsafe {
            self.context
                .device
                .cmd_set_scissor(self.handle, 0, &[vk::Rect2D { offset, extent }]);
        }
    }

    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.context.device.cmd_draw_indexed(
                self.handle,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            );
        }
    }
}