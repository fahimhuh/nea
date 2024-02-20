use crate::{
    render::frame::FrameRef,
    vulkan::{buffer::Buffer, context::Context},
    world::World,
};
use ash::vk;
use std::sync::Arc;

#[repr(C)]
// The ShaderUniforms struct is a representation of the data that is sent to the GPU
// and is used by the raytracing compute shader.
// It is updated each frame so we need to have multiple copies of it for each frame
// in flight to prevent race conditions on the GPU.
// It is represented as a C struct as the Vulkan API is a C API and the data is sent to the GPU
// The data within the struct is layed out specifically, to have all the data aligned to 16 bytes,
// which is essential for cache coherency. It reduces read latency as all the data for a single variable
// can be read in one GPU read operation, which is much faster than reading multiple times.
struct ShaderUniforms {
    // First 16 bytes

    // Random seed for the random number generator
    seed: u32,
    // Number of samples to take per pixel
    samples: u32,
    // Number of bounces to take per ray
    bounces: u32,
    // Padding to align to 16 bytes
    dummy: u32,

    // Second 16 bytes
    // Camera position
    pos: glam::Vec3A,

    // Matrices are 64 bytes each
    // which are a nice multiple of 16 anyway
    // so no padding is needed

    // View matrix
    inv_view: glam::Mat4,

    // Projection matrix
    inv_proj: glam::Mat4,
}

pub struct Uniforms {
    buffers: Vec<Buffer>,
}

impl Uniforms {
    pub const UNIFORMS_SIZE: u64 = std::mem::size_of::<ShaderUniforms>() as u64;

    pub fn new(context: Arc<Context>) -> Self {
        // Allocate uniform buffers for each frame in flight (Which is hardcoded to 3)
        let mut buffers = Vec::new();
        for i in 0..3 {
            buffers.push(Buffer::new(
                context.clone(),
                Self::UNIFORMS_SIZE,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                gpu_allocator::MemoryLocation::CpuToGpu,
                &format!("Raytracing Uniform Buffer {}", i),
            ))
        }

        Self { buffers }
    }

    pub fn update_uniforms(&mut self, frame: &FrameRef, world: &World) -> &Buffer {
        let buffer = &self.buffers[frame.index()];
        let ptr = buffer.get_ptr().cast::<ShaderUniforms>().as_ptr();

        // Calculate the aspect ratio of the display
        let aspect = frame.display.dims.as_vec2();
        let aspect_ratio = aspect.x / aspect.y;

        // Generate a random seed for the random number generator
        let seed = rand::random();

        // Calculate the forward and up vectors of the camera given the rotation quaternion
        let forward = world.camera.rotation * glam::vec3(0.0, 0.0, 1.0);
        let up = world.camera.rotation * glam::vec3(0.0, 1.0, 0.0);

        unsafe {
            // Write the uniforms to the GPU buffer
            ptr.write(ShaderUniforms {
                seed,
                samples: world.settings.samples,
                bounces: world.settings.bounces,
                dummy: 0,

                pos: world.camera.position.into(),

                // CALCULATE THE INVERSE VIEW MATRIX
                inv_view: glam::Mat4::look_to_lh(world.camera.position.into(), forward, up)
                    .inverse(),

                // CALCULATE THE INVERSE PROJECTION MATRIX
                inv_proj: glam::Mat4::perspective_lh(
                    f32::to_radians(world.settings.fov),
                    aspect_ratio,
                    world.settings.near,
                    world.settings.far,
                )
                .inverse(),
            })
        };

        buffer
    }
}
