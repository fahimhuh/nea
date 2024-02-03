use std::{ptr, sync::Arc};

use crate::{
    loader::{images::GpuImage, objects::GpuObject, SceneData},
    vulkan::{
        buffer::Buffer,
        command::CommandPool,
        context::Context,
        image::Image,
        rt::{AccelerationStructure, GeometryDescription, GeometryInstance},
        sync::Fence,
    },
};
use ash::vk::{self, BufferImageCopy};
use glam::Vec3Swizzles;

pub struct Texture {
    image: Image,
    dims: glam::UVec2,
    format: vk::Format,
}

pub struct Mesh {
    vertices: Buffer,
    indices: Buffer,
    blas: AccelerationStructure,
}

#[repr(C)]
pub struct Material {
    base_color: glam::Vec3A,
    emissive: glam::Vec3A,
    roughness: f32,
    metallic: f32,
}

pub struct Scene {
    pub textures: Vec<Texture>,
    pub meshes: Vec<Mesh>,
    pub materials: Buffer,

    pub tlas: AccelerationStructure,
}

impl Scene {
	pub const MATERIAL_BUFFER_SIZE: u64 = (std::mem::size_of::<Material>() * 4096) as u64;


    pub fn load(context: Arc<Context>, data: SceneData) -> Self {
        let command_pool = CommandPool::new(context.clone(), context.queue_family);
        let textures = Self::upload_textures(&context, &command_pool, data.images);
		let meshes = Self::build_meshes(&context, &command_pool, &data.objects);
		let materials = Self::upload_materials(&context, &data.objects);

		let tlas = Self::build_tlas(&context, &command_pool, &data.objects, &meshes);

		Self {
			textures,
			meshes,
			materials,
			tlas
		}
	}

    fn upload_textures(
        context: &Arc<Context>,
        command_pool: &CommandPool,
        images: Vec<GpuImage>,
    ) -> Vec<Texture> {
        let mut textures = Vec::with_capacity(images.len());
        for (index, image) in images.into_iter().enumerate() {
            let texture = Image::new(
                context.clone(),
                image.dims,
                image.format,
                vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                &format!("Scene Texture {}", index),
            );

            let staging = Buffer::new(
                context.clone(),
                image.bytes.len() as u64,
                vk::BufferUsageFlags::TRANSFER_SRC,
                gpu_allocator::MemoryLocation::CpuToGpu,
                &format!("Staging buffer for Texture {}", index),
            );

            let fence = Fence::new(context.clone(), false);

            let ptr = staging.get_ptr().cast::<u8>().as_ptr();
            unsafe { ptr.copy_from_nonoverlapping(image.bytes.as_ptr(), image.bytes.len()) };

            let cmds = command_pool.allocate();
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
            cmds.copy_to_image(&staging, &texture, &[copy]);

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

            context.submit(&[cmds], None, None, Some(&fence));
            fence.wait_and_reset();
            textures.push(Texture {
                image: texture,
                dims: image.dims.xy(),
                format: image.format,
            });
        }
        textures
    }

    fn build_meshes(context: &Arc<Context>, command_pool: &CommandPool, objects: &Vec<GpuObject>) -> Vec<Mesh> {
        let mut descs = Vec::new();
        let mut buffer_pairs = Vec::new();
        for (_index, object) in objects.iter().enumerate() {
            // Create staging buffers that are accesible by the CPU
            let indices_staging = Buffer::new(
                context.clone(),
                (object.indices.len() * std::mem::size_of::<u32>()) as u64,
                vk::BufferUsageFlags::TRANSFER_SRC,
                gpu_allocator::MemoryLocation::CpuToGpu,
                "Mesh Indices Staging Buffer",
            );

            let vertices_staging = Buffer::new(
                context.clone(),
                (object.vertices.len() * std::mem::size_of::<f32>()) as u64,
                vk::BufferUsageFlags::TRANSFER_SRC,
                gpu_allocator::MemoryLocation::CpuToGpu,
                "Mesh Vertices Staging Buffer",
            );

            // Create buffers that are stored on GPU VRAM
            let vertices = Buffer::new(
                context.clone(),
                (object.vertices.len() * std::mem::size_of::<f32>()) as u64,
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                gpu_allocator::MemoryLocation::GpuOnly,
                "Vertex Buffer",
            );

            let indices = Buffer::new(
                context.clone(),
                (object.indices.len() * std::mem::size_of::<u32>()) as u64,
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                gpu_allocator::MemoryLocation::GpuOnly,
                "Index Buffer",
            );

            let fence = Fence::new(context.clone(), false);

            // Copy the data from the buffers into the staging buffer
            unsafe {
                ptr::copy_nonoverlapping(
                    object.indices.as_ptr(),
                    indices_staging.get_ptr().cast::<u32>().as_ptr(),
                    object.indices.len(),
                );
                ptr::copy_nonoverlapping(
                    object.vertices.as_ptr(),
                    vertices_staging.get_ptr().cast::<f32>().as_ptr(),
                    object.vertices.len(),
                );
            }

            let cmds = command_pool.allocate();
            let vertices_region = vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: (object.vertices.len() * std::mem::size_of::<f32>()) as u64,
            };

            let indices_region = vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: (object.indices.len() * std::mem::size_of::<u32>()) as u64,
            };

            cmds.begin();

            cmds.copy_buffer(&vertices_staging, &vertices, &[vertices_region]);
            cmds.copy_buffer(&indices_staging, &indices, &[indices_region]);

            cmds.end();

            context.submit(&[cmds], None, None, Some(&fence));
            fence.wait_and_reset();

            let desc = GeometryDescription {
                vertices: vertices.get_addr(),
                indices: indices.get_addr(),
                max_vertex: (object.vertices.len() - 1) as u32,
                primitives: object.indices.len().div_ceil(3) as u32,
            };

            descs.push(desc);

            buffer_pairs.push((vertices, indices));
        }

        let blasses = AccelerationStructure::build_bottom_levels(context.clone(), &descs);

        let meshes = blasses
            .into_iter()
            .zip(buffer_pairs)
            .map(|(blas, (vertices, indices))| Mesh {
                vertices,
                indices,
                blas,
            })
            .collect::<Vec<Mesh>>();

		meshes
    }

	fn upload_materials(context: &Arc<Context>, objects: &Vec<GpuObject>) -> Buffer {

		let material_buffer = Buffer::new(
            context.clone(),
            Self::MATERIAL_BUFFER_SIZE,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            gpu_allocator::MemoryLocation::CpuToGpu,
            &format!("Material Buffer"),
        );

		for (index, object) in objects.iter().enumerate() {
			let ptr = unsafe {
                material_buffer
                    .get_ptr()
                    .cast::<Material>()
                    .as_ptr()
                    .offset(index as isize)
            };

            let material = Material {
                base_color: object.base_color,
                emissive: object.emissive,
                roughness: object.roughness,
                metallic: object.metallic,
            
			};
            unsafe { ptr.write(material) };
		}

		material_buffer
	}

	fn build_tlas(context: &Arc<Context>, _command_pool: &CommandPool, objects: &Vec<GpuObject>, meshes: &[Mesh]) -> AccelerationStructure {
		let mut instances = Vec::with_capacity(objects.len());

        for (index, object) in objects.iter().enumerate() {
			let instance = GeometryInstance {
                transform: object.transform,
                blas: meshes[index].blas.get_addr(),
                index: index as u32,
            };

            instances.push(instance)
        }
		
		let tlas = AccelerationStructure::build_top_level(context.clone(), &instances);
		tlas
	}
}
