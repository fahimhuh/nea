use crate::{
    loader::{GpuObject, SceneData},
    vulkan::{
        buffer::Buffer,
        command::CommandPool,
        context::Context,
        rt::{AccelerationStructure, GeometryDescription, GeometryInstance},
        sync::Fence,
    },
};
use ash::vk;
use std::{ptr, sync::Arc};

// The mesh struct is a representation of a mesh in the scene
// and contains the vertex and index buffers for the mesh
// as well as the acceleration structure for the mesh
// and it is all stored on the GPU VRAM for fast access.
pub struct Mesh {
    vertices: Buffer,
    indices: Buffer,
    blas: AccelerationStructure,
}

#[repr(C)]
// The Material struct is a representation of the material of an object in the scene
// and is used by the raytracing compute shader. It should be in the same layout 
// as the Material struct in the raytracer.comp shader.
pub struct Material {
    base_color: glam::Vec3A,
    emissive: glam::Vec3A,
    roughness: f32,
    metallic: f32,
}

// The Scene struct is a representation of the scene and is used by the raytracing compute shader
// It contains the meshes, materials and acceleration structures for the scene and is stored on the GPU VRAM
// it is a reflection of the world, but is a more efficient representation of the world
pub struct Scene {
    // The meshes in the scene
    pub meshes: Vec<Mesh>,
    // The materials in the scene
    pub materials: Buffer,

    // The top level acceleration structure for the scene which contains all the meshes
    pub tlas: AccelerationStructure,
}

impl Scene {
    // The size of the material buffer
    pub const MATERIAL_BUFFER_SIZE: u64 = (std::mem::size_of::<Material>() * 4096) as u64;

    // Load the scene from the scene data
    pub fn load(context: Arc<Context>, data: SceneData) -> Self {
        // Create a command pool to allocate command buffers from
        let command_pool = CommandPool::new(context.clone(), context.queue_family);
        // Build the meshes for the scene
        let meshes = Self::build_meshes(&context, &command_pool, &data.objects);
        // Upload the materials for the scene
        let materials = Self::upload_materials(&context, &data.objects);

        // Build the top level acceleration structure for the scene
        let tlas = Self::build_tlas(&context, &command_pool, &data.objects, &meshes);

        Self {
            meshes,
            materials,
            tlas,
        }
    }

    // Build the meshes for the scene given the objects in the scene
    fn build_meshes(
        context: &Arc<Context>,
        command_pool: &CommandPool,
        objects: &Vec<GpuObject>,
    ) -> Vec<Mesh> {
    
        let mut descs = Vec::new();
        let mut buffer_pairs = Vec::new();
    
        for  object in objects {
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

            // Allocate a command buffer to copy the data from the staging buffer to the GPU VRAM
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
            // Copy the data from the staging buffer to the GPU VRAM
            cmds.copy_buffer(&vertices_staging, &vertices, &[vertices_region]);
            cmds.copy_buffer(&indices_staging, &indices, &[indices_region]);

            cmds.end();

            // Submit the command buffer to the queue
            context.submit(&[cmds], None, None, Some(&fence));
            // Wait for the command buffer to finish executing
            fence.wait_and_reset();

            // Create a description of the geometry for the acceleration structure
            let desc = GeometryDescription {
                vertices: vertices.get_addr(),
                indices: indices.get_addr(),
                max_vertex: (object.vertices.len() - 1) as u32,
                primitives: object.indices.len().div_ceil(3) as u32,
            };

            // add the description and the buffers to the list of descriptions and buffer pairs
            descs.push(desc);
            buffer_pairs.push((vertices, indices));
        }

        // Build the bottom level acceleration structures for the meshes
        let blasses = AccelerationStructure::build_bottom_levels(context.clone(), &descs);

        // Create the mesh objects using a functional iterator
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

    // Upload the materials for the scene
    fn upload_materials(context: &Arc<Context>, objects: &Vec<GpuObject>) -> Buffer {
        // Create a buffer to store the materials on the GPU VRAM
        let material_buffer = Buffer::new(
            context.clone(),
            Self::MATERIAL_BUFFER_SIZE,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            gpu_allocator::MemoryLocation::CpuToGpu,
            &format!("Material Buffer"),
        );

        // Copy the materials from the objects into the buffer
        for (index, object) in objects.iter().enumerate() {
            // Get a pointer to the material in the buffer
            let ptr = unsafe {
                material_buffer
                    .get_ptr()
                    .cast::<Material>()
                    .as_ptr()
                    // Offset the pointer by the index of the object
                    .offset(index as isize)
            };

            // Create a material from the object
            let material = Material {
                base_color: object.base_color,
                emissive: object.emissive,
                roughness: object.roughness,
                metallic: object.metallic,
            };

            // Write the material to the buffer
            unsafe { ptr.write(material) };
        }

        material_buffer
    }

    fn build_tlas(
        context: &Arc<Context>,
        _command_pool: &CommandPool,
        objects: &Vec<GpuObject>,
        meshes: &[Mesh],
    ) -> AccelerationStructure {
        // Create a list of instances for the top level acceleration structure
        let mut instances = Vec::with_capacity(objects.len());

        // Create an instance for each object in the scene
        for (index, object) in objects.iter().enumerate() {
            let instance = GeometryInstance {
                transform: object.transform,
                blas: meshes[index].blas.get_addr(),
                index: index as u32,
            };

            instances.push(instance)
        }

        // Build the top level acceleration structure
        let tlas = AccelerationStructure::build_top_level(context.clone(), &instances);
        tlas
    }
}
