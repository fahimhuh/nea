use std::sync::Arc;

use ash::vk;

use crate::{
    render::frame::FrameRef,
    vulkan::{buffer::Buffer, context::Context},
    world::World,
};

#[repr(C)]
struct ShaderUniforms {
    seed: u32,
    samples: u32,
    bounces: u32,
    mode: u32,

    focal_length: f32,
    aperture: f32,
    exposure: f32,
    time: f32,

    // Camera position
    pos: glam::Vec3A,

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

        let aspect = frame.display.dims.as_vec2();
        let aspect_ratio = aspect.x / aspect.y;

        let seed = rand::random();

        let forward = world.camera.rotation * glam::vec3(0.0, 0.0, 1.0);
        let up = world.camera.rotation * glam::vec3(0.0, 1.0, 0.0);

        unsafe {
            ptr.write(ShaderUniforms {
                seed,
                samples: world.settings.samples,
                bounces: world.settings.bounces,
                mode: 0,

                focal_length: world.settings.focal_length,
                aperture: world.settings.aperture,
                exposure: world.settings.exposure,
                time: 0.0,

                pos: world.camera.position.into(),

                inv_view: glam::Mat4::look_to_lh(world.camera.position.into(), forward, up)
                    .inverse(),
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
