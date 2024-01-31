use super::frame::FrameRef;
use crate::{
    loader::{images::GpuImage, objects::GpuObject, SceneData, SceneLoader},
    vulkan::{
        buffer::Buffer,
        command::{CommandList, CommandPool},
        context::Context,
        image::Image,
        rt::{AccelerationStructure, GeometryDescription, GeometryInstance},
        sync::Fence,
    },
    world::World,
};
use ash::vk::{self, BufferImageCopy};
use glam::Vec3Swizzles;
use std::{cmp::max, sync::Arc, time::Instant};

pub struct Texture {
    image: Image,
    dims: glam::UVec2,
    format: vk::Format,
}

pub struct Mesh {
    vertices: Buffer,
    indices: Buffer,
}

pub struct Raytracer {
    command_pool: CommandPool,

    textures: Vec<Texture>,
    meshes: Vec<Mesh>,

    blasses: Vec<AccelerationStructure>,
    tlas: Option<AccelerationStructure>,
}

impl Raytracer {
    pub fn new(context: Arc<Context>) -> Self {
        let command_pool = CommandPool::new(context.clone(), context.queue_family);
        let textures = Vec::new();
        let meshes = Vec::new();

        let blasses = Vec::new();
        let tlas = None;

        Self {
            command_pool,
            textures,
            meshes,
            blasses,
            tlas,
        }
    }

    pub fn run(&mut self, _commands: &CommandList, frame: &FrameRef, _world: &World) {
        if let Some(scene) = SceneLoader::poll() {
            log::info!("Loading scene into GPU memory");
            self.load_scene(frame, scene);
        }
    }

    fn load_scene(&mut self, frame: &FrameRef, scene: SceneData) {
        self.load_textures(frame, scene.images);
        self.load_objects(frame, scene.objects);
        self.command_pool.reset();
    }

    fn load_textures(&mut self, frame: &FrameRef, textures: Vec<GpuImage>) {
        self.textures.clear();
        let start = Instant::now();
        for (index, image) in textures.into_iter().enumerate() {
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

        let time = Instant::now() - start;
        log::info!(
            "Loaded {} textures in {:?}",
            self.textures.len(),
            time
        );
    }

    fn load_objects(&mut self, frame: &FrameRef, objects: Vec<GpuObject>) {
        self.meshes.clear();

        let start = Instant::now();
        let mut geometries = Vec::with_capacity(objects.len());
        for object in &objects {
            let size = max(
                object.indices.len() * std::mem::size_of::<u32>(),
                object.vertices.len() * std::mem::size_of::<f32>(),
            ) as u64;

            let staging = Buffer::new(
                frame.context.clone(),
                size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                gpu_allocator::MemoryLocation::CpuToGpu,
                "Mesh Staging Buffer",
            );

            let vertices = Buffer::new(
                frame.context.clone(),
                (object.vertices.len() * std::mem::size_of::<f32>()) as u64,
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                gpu_allocator::MemoryLocation::GpuOnly,
                "Vertex Buffer",
            );

            let indices = Buffer::new(
                frame.context.clone(),
                (object.indices.len() * std::mem::size_of::<u32>()) as u64,
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                gpu_allocator::MemoryLocation::GpuOnly,
                "Index Buffer",
            );

            let fence = Fence::new(frame.context.clone(), false);

            let ptr = staging.get_ptr().cast::<f32>().as_ptr();
            unsafe { ptr.copy_from_nonoverlapping(object.vertices.as_ptr(), object.vertices.len()) }

            let cmds = self.command_pool.allocate();
            let region = vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: (object.vertices.len() * std::mem::size_of::<f32>()) as u64,
            };

            cmds.begin();
            cmds.copy_buffer(&staging, &vertices, &[region]);
            cmds.end();
            frame.context.submit(&[cmds], None, None, Some(&fence));
            fence.wait_and_reset();

            let ptr = staging.get_ptr().cast::<u32>().as_ptr();
            unsafe { ptr.copy_from_nonoverlapping(object.indices.as_ptr(), object.vertices.len()) }

            let cmds = self.command_pool.allocate();
            let region = vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: (object.indices.len() * std::mem::size_of::<u32>()) as u64,
            };

            cmds.begin();
            cmds.copy_buffer(&staging, &vertices, &[region]);
            cmds.end();
            frame.context.submit(&[cmds], None, None, Some(&fence));
            fence.wait_and_reset();

            let geometry = GeometryDescription {
                vertices: vertices.get_addr(),
                indices: indices.get_addr(),
                max_vertex: (object.vertices.len() - 1) as u32,
                primitives: object.indices.len().div_ceil(3) as u32,
            };

            geometries.push(geometry);

            let mesh = Mesh { vertices, indices };

            self.meshes.push(mesh);
        }
        log::info!("Loaded {} meshes; Building meshes...", self.meshes.len());

        let blasses =
            AccelerationStructure::build_bottom_levels(frame.context.clone(), &geometries);
        log::info!("Built meshes, building scene..");

        let mut instances = Vec::with_capacity(objects.len());
        for (index, object) in objects.iter().enumerate() {
            let instance = GeometryInstance {
                transform: object.transform,
                blas: blasses[index].get_addr(),
            };

            instances.push(instance)
        }

        let tlas = AccelerationStructure::build_top_level(frame.context.clone(), &instances);

        self.blasses.clear();
        self.blasses = blasses;

        self.tlas = Some(tlas);

        let time = Instant::now() - start;
        log::info!("Scene built in {:?}", time);
    }
}
