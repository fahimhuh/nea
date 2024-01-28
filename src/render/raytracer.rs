use super::frame::FrameRef;
use crate::{
    interface::Interface,
    loader::{SceneData, SceneLoader},
    vulkan::{
        buffer::Buffer,
        command::{CommandList, CommandPool},
        context::Context,
        image::Image,
        sync::Fence,
    },
    world::World,
};
use ash::vk::{self, BufferImageCopy};
use glam::Vec3Swizzles;
use std::sync::Arc;

pub struct Texture {
    image: Image,
    dims: glam::UVec2,
    format: vk::Format,
}

pub struct Raytracer {
    command_pool: CommandPool,
    textures: Vec<Texture>,
}

impl Raytracer {
    pub fn new(context: Arc<Context>) -> Self {
        let command_pool = CommandPool::new(context.clone(), context.queue_family);
        let textures = Vec::new();
        Self {
            command_pool,
            textures,
        }
    }

    pub fn run(&mut self, commands: &CommandList, frame: &FrameRef, world: &World) {
        if let Some(scene) = SceneLoader::poll() {
            log::info!("Loading scene into GPU memory");
            self.load_scene(frame, scene);
        }
    }

    fn load_scene(&mut self, frame: &FrameRef, scene: SceneData) {
        self.textures.clear();
        for (index, image) in scene.images.into_iter().enumerate() {
            let texture = Image::new(
                frame.context.clone(),
                image.dims,
                image.format,
                vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                &format!("Scene Texture {}", index),
            );
            let buffer = Buffer::new(
                frame.context.clone(),
                image.bytes.len() as u64,
                vk::BufferUsageFlags::TRANSFER_SRC,
                gpu_allocator::MemoryLocation::CpuToGpu,
                &format!("Staging buffer for Texture {}", index),
            );
            let fence = Fence::new(frame.context.clone(), false);

            let ptr = buffer.get_ptr().cast::<u8>().as_ptr();
            unsafe { ptr.copy_from_nonoverlapping(image.bytes.as_ptr(), image.bytes.len()) };

            let cmds = self.command_pool.allocate();
            cmds.begin();

            let barrier = [vk::ImageMemoryBarrier2 {
                src_stage_mask: vk::PipelineStageFlags2::NONE,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: texture.handle,
                subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
                ..Default::default()
            }];

            cmds.pipeline_barrier(&barrier, &[]);

            let copy = BufferImageCopy {
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
                    width: image.dims.x,
                    height: image.dims.y,
                    depth: image.dims.z,
                },
            };
            cmds.copy_to_image(&buffer, &texture, &[copy]);

            let barrier = [vk::ImageMemoryBarrier2 {
                src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                dst_stage_mask: vk::PipelineStageFlags2::COMPUTE_SHADER,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: texture.handle,
                subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
                ..Default::default()
            }];

            cmds.pipeline_barrier(&barrier, &[]);
            cmds.end();

            frame.context.submit(&[cmds], None, None, Some(&fence));
            fence.wait_and_reset();
            self.textures.push(Texture {
                image: texture,
                dims: image.dims.xy(),
                format: image.format,
            });
        }

        log::info!("Loaded {} textures successfully", self.textures.len());
    }
}
