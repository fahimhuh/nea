use super::frame::FrameRef;
use crate::{
    interface::Interface,
    vulkan::{
        buffer::Buffer,
        command::{CommandList, CommandPool},
        context::Context,
        descriptor::{
            DescriptorBinding, DescriptorImageWrite, DescriptorPool, DescriptorSet,
            DescriptorSetLayout,
        },
        display::Display,
        image::{Image, ImageView, Sampler},
        pipeline::{GraphicsPipeline, PipelineLayout},
        shader::Shader,
        sync::Fence,
    },
};
use ash::vk;
use egui::{epaint::ImageDelta, TextureId};
use std::{collections::HashMap, sync::Arc};

mod shaders {
    pub mod vertex {
        include!(concat!(env!("OUT_DIR"), "/interface.vert.rs"));
    }
    pub mod fragment {
        include!(concat!(env!("OUT_DIR"), "/interface.frag.rs"));
    }
}

pub struct EguiFrameData {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

pub struct EguiTextureData {
    image: Image,
    view: ImageView,
    descriptor_set: DescriptorSet,
    dims: glam::UVec3,
}

#[derive(bytemuck::Zeroable, bytemuck::Pod, Clone, Copy)]
#[repr(C)]
pub struct EguiPushConstants {
    dimensions: glam::Vec2,
}

pub struct InterfacePainter {
    transfer_command_pool: CommandPool,
    descriptor_pool: DescriptorPool,
    descriptor_set_layout: DescriptorSetLayout,
    pipeline_layout: PipelineLayout,
    pipeline: GraphicsPipeline,
    sampler: Sampler,
    frame_data: Vec<EguiFrameData>,
    textures: HashMap<egui::TextureId, EguiTextureData>,
}

impl InterfacePainter {
    const VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 4;
    const INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 2;

    pub fn new(context: Arc<Context>, display: &Display) -> Self {
        let transfer_command_pool = CommandPool::new(context.clone(), context.queue_family);

        let descriptor_pool = DescriptorPool::new(context.clone());

        let bindings = vec![DescriptorBinding {
            binding: 0,
            count: 1,
            kind: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            stage: vk::ShaderStageFlags::FRAGMENT,
        }];

        let descriptor_set_layout = DescriptorSetLayout::new(context.clone(), bindings);

        let push_constants = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: std::mem::size_of::<f32>() as u32 * 2,
        };

        let pipeline_layout = PipelineLayout::new(
            context.clone(),
            push_constants,
            std::slice::from_ref(&descriptor_set_layout),
        );

        let vert_code = shaders::vertex::CODE;
        let vert_shader = Shader::new(
            context.clone(),
            &vert_code,
            vk::ShaderStageFlags::VERTEX,
            "main",
        );

        let frag_code = shaders::fragment::CODE;
        let frag_shader = Shader::new(
            context.clone(),
            &frag_code,
            vk::ShaderStageFlags::FRAGMENT,
            "main",
        );

        let binding = vk::VertexInputBindingDescription {
            binding: 0,
            stride: (4 * std::mem::size_of::<f32>() + 4 * std::mem::size_of::<u8>()) as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        };

        let attributes = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            }, // Position
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 8,
            }, // UV
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R8G8B8A8_UNORM,
                offset: 16,
            }, // Color
        ];

        let vertex_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&attributes)
            .vertex_binding_descriptions(std::slice::from_ref(&binding))
            .build();

        let render_info = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(std::slice::from_ref(&display.format))
            .build();

        let pipeline = GraphicsPipeline::new(
            context.clone(),
            &vert_shader,
            &frag_shader,
            &pipeline_layout,
            vertex_info,
            render_info,
        );

        let mut frame_data = Vec::new();

        for i in 0..display.frames_in_flight() {
            let frame = EguiFrameData {
                vertex_buffer: Buffer::new(
                    context.clone(),
                    Self::VERTEX_BUFFER_SIZE,
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                    gpu_allocator::MemoryLocation::CpuToGpu,
                    &format!("UI Vertex Buffer {}", i),
                ),
                index_buffer: Buffer::new(
                    context.clone(),
                    Self::INDEX_BUFFER_SIZE,
                    vk::BufferUsageFlags::INDEX_BUFFER,
                    gpu_allocator::MemoryLocation::CpuToGpu,
                    &format!("UI Index Buffer {}", i),
                ),
            };

            frame_data.push(frame);
        }

        let sampler = Sampler::new(
            context.clone(),
            vk::SamplerAddressMode::CLAMP_TO_EDGE,
            vk::Filter::LINEAR,
        );

        Self {
            transfer_command_pool,
            descriptor_pool,
            descriptor_set_layout,
            pipeline_layout,
            pipeline,
            frame_data,
            textures: HashMap::new(),
            sampler,
        }
    }

    pub fn draw(&mut self, cmds: &CommandList, frame: &FrameRef, interface: &mut Interface) {
        let output = interface.take_last_output();

        // Update textures
        for (id, delta) in output.textures_delta.set {
            self.update_texture(frame, id, delta);
        }

        let image_memory_barriers = [vk::ImageMemoryBarrier2 {
            src_stage_mask: vk::PipelineStageFlags2::NONE,
            src_access_mask: vk::AccessFlags2::NONE,
            dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: frame.image().handle,
            subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
            ..Default::default()
        }];

        cmds.pipeline_barrier(&image_memory_barriers, &[]);

        let swapchain_attachment = vk::RenderingAttachmentInfo::builder()
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image_view(frame.image_view().handle)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue::default());

        let rendering_info = vk::RenderingInfo::builder()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: vk::Extent2D {
                    width: frame.display.dims.x,
                    height: frame.display.dims.y,
                },
            })
            .color_attachments(std::slice::from_ref(&swapchain_attachment))
            .layer_count(1)
            .build();

        let frame_data = self.frame_data.get_mut(frame.index()).unwrap();

        cmds.begin_rendering(rendering_info);
        cmds.bind_graphics_pipeline(&self.pipeline);
        cmds.bind_vertex_buffer(&frame_data.vertex_buffer);
        cmds.bind_index_buffer(&frame_data.index_buffer);
        cmds.set_viewport(
            0.0,
            0.0,
            frame.display.dims.x as f32,
            frame.display.dims.y as f32,
        );

        let push_constants = frame.display.dims.as_vec2() / frame.display.dpi;
        cmds.push_constants(
            &self.pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            push_constants,
        );

        let mut vertex_ptr = frame_data
            .vertex_buffer
            .get_ptr()
            .cast::<egui::epaint::Vertex>()
            .as_ptr();
        let mut index_ptr = frame_data.index_buffer.get_ptr().cast::<u32>().as_ptr();

        let mut base_vertex = 0;
        let mut base_index = 0;

        let primitives = interface
            .context()
            .tessellate(output.shapes, frame.display.dpi);

        for primitive in primitives {
            let mesh = match primitive.primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh,
                _ => unimplemented!(),
            };

            unsafe {
                vertex_ptr.copy_from_nonoverlapping(mesh.vertices.as_ptr(), mesh.vertices.len());
                vertex_ptr = vertex_ptr.add(mesh.vertices.len());

                index_ptr.copy_from_nonoverlapping(mesh.indices.as_ptr(), mesh.indices.len());
                index_ptr = index_ptr.add(mesh.indices.len());
            }

            let texture_data = self.textures.get(&mesh.texture_id).unwrap();
            cmds.bind_descriptor_sets(
                vk::PipelineBindPoint::GRAPHICS,
                &self.pipeline_layout,
                &[texture_data.descriptor_set.handle],
            );

            let min = (glam::vec2(primitive.clip_rect.min.x, primitive.clip_rect.min.y)
                * frame.display.dpi)
                .as_ivec2();
            let offset = vk::Offset2D { x: min.x, y: min.y };

            let extent = vk::Extent2D {
                width: primitive.clip_rect.width().round() as u32,
                height: primitive.clip_rect.height().round() as u32,
            };
            cmds.set_scissor(offset, extent);

            cmds.draw_indexed(
                mesh.indices.len() as u32,
                1,
                base_index as u32,
                base_vertex as i32,
                0,
            );

            base_vertex += mesh.vertices.len();
            base_index += mesh.indices.len();
        }
        cmds.end_rendering();

        let image_memory_barriers = [vk::ImageMemoryBarrier2 {
            src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags2::NONE,
            dst_access_mask: vk::AccessFlags2::NONE,
            old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: frame.display.images[frame.index()].handle,
            subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
            ..Default::default()
        }];
        cmds.pipeline_barrier(&image_memory_barriers, &[]);

        // Destroy what needs to be destroyed
        for id in output.textures_delta.free {
            self.textures.remove(&id);
        }
    }

    pub fn update_texture(&mut self, frame: &FrameRef, id: TextureId, delta: ImageDelta) {
        let bytes: Vec<u8> = match &delta.image {
            egui::ImageData::Color(image) => {
                image.pixels.iter().flat_map(|c| c.to_array()).collect()
            }
            egui::ImageData::Font(image) => image
                .srgba_pixels(None)
                .flat_map(|c| c.to_array())
                .collect(),
        };

        let staging = Buffer::new(
            frame.context.clone(),
            bytes.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            gpu_allocator::MemoryLocation::CpuToGpu,
            "UI Staging Buffer",
        );
        unsafe {
            staging
                .get_ptr()
                .cast::<u8>()
                .as_ptr()
                .copy_from_nonoverlapping(bytes.as_ptr(), bytes.len())
        };

        let image = Image::new(
            frame.context.clone(),
            glam::uvec3(delta.image.width() as u32, delta.image.height() as u32, 1),
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::TRANSFER_SRC,
            "UI Texture",
        );

        let view = ImageView::new(
            frame.context.clone(),
            &image,
            vk::Format::R8G8B8A8_UNORM,
            Image::default_subresource(vk::ImageAspectFlags::COLOR),
        );

        let cmds = self.transfer_command_pool.allocate();
        cmds.begin();
        let barrier = [vk::ImageMemoryBarrier2 {
            src_stage_mask: vk::PipelineStageFlags2::HOST,
            src_access_mask: vk::AccessFlags2::NONE,
            dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
            dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: image.handle,
            subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
            ..Default::default()
        }];

        cmds.pipeline_barrier(&barrier, &[]);

        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D::default(),
            image_extent: vk::Extent3D {
                width: delta.image.width() as u32,
                height: delta.image.height() as u32,
                depth: 1,
            },
        };

        cmds.copy_to_image(&staging, &image, &[region]);

        let barrier = [vk::ImageMemoryBarrier2 {
            src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
            src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
            dst_stage_mask: vk::PipelineStageFlags2::VERTEX_SHADER,
            dst_access_mask: vk::AccessFlags2::SHADER_READ,
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: image.handle,
            subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
            ..Default::default()
        }];

        cmds.pipeline_barrier(&barrier, &[]);
        cmds.end();

        let fence = Fence::new(frame.context.clone(), false);
        frame.context.submit(&[cmds], None, None, Some(&fence));
        fence.wait_and_reset();

        if let Some(pos) = delta.pos {
            let cmds = self.transfer_command_pool.allocate();
            cmds.begin();
            let texture_data = self.textures.get(&id).unwrap();
            let barriers = [
                // Existing Texture
                vk::ImageMemoryBarrier2 {
                    src_stage_mask: vk::PipelineStageFlags2::FRAGMENT_SHADER,
                    src_access_mask: vk::AccessFlags2::SHADER_READ,
                    dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                    old_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                    image: texture_data.image.handle,
                    subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
                    ..Default::default()
                },
                // New texture we just created
                vk::ImageMemoryBarrier2 {
                    src_stage_mask: vk::PipelineStageFlags2::FRAGMENT_SHADER,
                    src_access_mask: vk::AccessFlags2::SHADER_READ,
                    dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    dst_access_mask: vk::AccessFlags2::TRANSFER_READ,
                    old_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                    image: image.handle,
                    subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
                    ..Default::default()
                },
            ];

            cmds.pipeline_barrier(&barriers, &[]);

            let top_left = vk::Offset3D {
                x: pos[0] as i32,
                y: pos[1] as i32,
                z: 0,
            };
            let bottom_right = vk::Offset3D {
                x: pos[0] as i32 + delta.image.width() as i32,
                y: pos[1] as i32 + delta.image.height() as i32,
                z: 1,
            };

            let region = vk::ImageBlit {
                src_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_offsets: [
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: delta.image.width() as i32,
                        y: delta.image.height() as i32,
                        z: 1 as i32,
                    },
                ],
                dst_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                dst_offsets: [top_left, bottom_right],
            };

            cmds.blit(&image, &texture_data.image, &[region]);

            let barrier = [vk::ImageMemoryBarrier2 {
                src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                dst_stage_mask: vk::PipelineStageFlags2::FRAGMENT_SHADER,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: texture_data.image.handle,
                subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
                ..Default::default()
            }];

            cmds.pipeline_barrier(&barrier, &[]);
            cmds.end();

            frame.context.submit(&[cmds], None, None, Some(&fence));
            fence.wait_and_reset();
        } else {
            let descriptor_set = self
                .descriptor_pool
                .allocate(&frame.context, &self.descriptor_set_layout, 1)
                .remove(0);

            descriptor_set.write(
                &[DescriptorImageWrite {
                    image_kind: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    sampler: Some(self.sampler.handle),
                    layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    image_view: &view,
                    binding: 0,
                }],
                &[],
            );

            let texture_data = EguiTextureData {
                image,
                view,
                descriptor_set,
                dims: glam::uvec3(delta.image.width() as u32, delta.image.width() as u32, 1),
            };

            self.textures.insert(id, texture_data);
        }
    }
}
