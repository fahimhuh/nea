use super::frame::FrameRef;
use crate::{
    loader::{images::GpuImage, objects::GpuObject, SceneData, SceneLoader},
    vulkan::{
        buffer::Buffer,
        command::{CommandList, CommandPool},
        context::Context,
        descriptor::{
            DescriptorBinding, DescriptorBufferWrite, DescriptorImageWrite, DescriptorPool,
            DescriptorSet, DescriptorSetLayout, DescriptorTLASWrite,
        },
        image::Image,
        pipeline::{ComputePipeline, PipelineLayout},
        rt::{AccelerationStructure, GeometryDescription, GeometryInstance},
        shader::Shader,
        sync::Fence,
    },
    world::{Camera, World},
};
use ash::vk::{self, BufferImageCopy};
use glam::Vec3Swizzles;
use std::{cmp::max, sync::Arc, time::Instant};

mod shader {
    include!(concat!(env!("OUT_DIR"), "/raytracer.comp.rs"));
}

pub struct Texture {
    image: Image,
    dims: glam::UVec2,
    format: vk::Format,
}

pub struct Mesh {
    vertices: Buffer,
    indices: Buffer,
}

#[repr(C)]
pub struct Material {
    base_color: glam::Vec3A,
    emissive: glam::Vec3A,
    roughness: f32,
    metallic: f32,
}

pub enum RenderMode {
    Full,
}

#[repr(C)]
pub struct UniformData {
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

pub struct Raytracer {
    command_pool: CommandPool,
    descriptor_pool: DescriptorPool,

    descriptor_layout: DescriptorSetLayout,
    pipeline_layout: PipelineLayout,

    shader: Shader,
    pipeline: ComputePipeline,

    descriptor_sets: Vec<DescriptorSet>,

    uniform_buffers: Vec<Buffer>,
    material_buffer: Buffer,

    textures: Vec<Texture>,
    meshes: Vec<Mesh>,

    blasses: Vec<AccelerationStructure>,
    tlas: Option<AccelerationStructure>,
}

impl Raytracer {
    pub fn new(context: Arc<Context>) -> Self {
        let command_pool = CommandPool::new(context.clone(), context.queue_family);
        let descriptor_pool = DescriptorPool::new(context.clone());

        let bindings = vec![
            DescriptorBinding {
                binding: 0,
                count: 1,
                kind: vk::DescriptorType::STORAGE_IMAGE,
                stage: vk::ShaderStageFlags::COMPUTE,
            },
            DescriptorBinding {
                binding: 1,
                count: 1,
                kind: vk::DescriptorType::UNIFORM_BUFFER,
                stage: vk::ShaderStageFlags::COMPUTE,
            },
            DescriptorBinding {
                binding: 2,
                count: 1,
                kind: vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
                stage: vk::ShaderStageFlags::COMPUTE,
            },
            DescriptorBinding {
                binding: 3,
                count: 1,
                kind: vk::DescriptorType::STORAGE_BUFFER,
                stage: vk::ShaderStageFlags::COMPUTE,
            },
        ];

        let descriptor_layout = DescriptorSetLayout::new(context.clone(), bindings);
        let push_constants = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            offset: 0,
            size: std::mem::size_of::<u64>() as u32,
        };

        let pipeline_layout = PipelineLayout::new(
            context.clone(),
            push_constants,
            std::slice::from_ref(&descriptor_layout),
        );

        let shader = Shader::new(
            context.clone(),
            &shader::CODE,
            vk::ShaderStageFlags::COMPUTE,
            "main",
        );

        let pipeline = ComputePipeline::new(context.clone(), &shader, &pipeline_layout);
        let descriptor_sets = descriptor_pool.allocate(&context, &descriptor_layout, 3);

        let mut uniform_buffers = Vec::new();
        for i in 0..3 {
            uniform_buffers.push(Buffer::new(
                context.clone(),
                std::mem::size_of::<UniformData>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                gpu_allocator::MemoryLocation::CpuToGpu,
                &format!("Raytracing Uniform Buffer {}", i),
            ))
        }

        let textures = Vec::new();
        let meshes = Vec::new();

        let blasses = Vec::new();
        let tlas = None;

        let material_buffer = Buffer::new(
            context.clone(),
            (std::mem::size_of::<Material>() * 4096) as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            gpu_allocator::MemoryLocation::CpuToGpu,
            &format!("Material Buffer"),
        );

        Self {
            command_pool,
            textures,
            meshes,
            blasses,
            tlas,
            descriptor_pool,
            descriptor_layout,
            pipeline_layout,
            shader,
            pipeline,
            descriptor_sets,
            uniform_buffers,
            material_buffer,
        }
    }

    pub fn run(&mut self, cmds: &CommandList, frame: &FrameRef, world: &World) {
        if let Some(scene) = SceneLoader::poll() {
            log::info!("Loading scene into GPU memory");
            self.load_scene(frame, scene);
        }

        // Only raytrace if there is a scene to trace against!
        if self.tlas.is_some() {
            self.raytrace(cmds, frame, world);
        }
    }

    fn raytrace(&mut self, cmds: &CommandList, frame: &FrameRef, world: &World) {
        let uniform_buffer = self.uniform_buffers.get(frame.index()).unwrap();
        self.update_uniform_buffer(frame, uniform_buffer, world);

        let descriptor_set = self.descriptor_sets.get(frame.index()).unwrap();
        descriptor_set.write(
            &[DescriptorImageWrite {
                image_view: frame.display.views.get(frame.index()).unwrap(),
                layout: vk::ImageLayout::GENERAL,
                binding: 0,
                sampler: None,
                image_kind: vk::DescriptorType::STORAGE_IMAGE,
            }],
            &[
                DescriptorBufferWrite {
                    buffer_kind: vk::DescriptorType::UNIFORM_BUFFER,
                    buffer: uniform_buffer,
                    range: std::mem::size_of::<UniformData>() as u64,
                    binding: 1,
                },
                DescriptorBufferWrite {
                    buffer_kind: vk::DescriptorType::STORAGE_BUFFER,
                    buffer: &self.material_buffer,
                    // TODO: Make material buffer size a constant
                    range: (std::mem::size_of::<Material>() * 4096) as u64,
                    binding: 3,
                },
            ],
        );

        descriptor_set.write_tlas(DescriptorTLASWrite {
            reference: self.tlas.as_ref().unwrap(),
            binding: 2,
        });

        let image_memory_barriers = [vk::ImageMemoryBarrier2 {
            src_stage_mask: vk::PipelineStageFlags2::NONE,
            src_access_mask: vk::AccessFlags2::NONE,
            dst_stage_mask: vk::PipelineStageFlags2::COMPUTE_SHADER,
            dst_access_mask: vk::AccessFlags2::SHADER_WRITE,
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::GENERAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: frame.display.images.get(frame.index()).unwrap().handle,
            subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
            ..Default::default()
        }];

        cmds.pipeline_barrier(&image_memory_barriers, &[]);

        cmds.bind_compute_pipeline(&self.pipeline);
        cmds.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.pipeline_layout,
            &[descriptor_set.handle],
        );

        cmds.dispatch(frame.display.dims.x, frame.display.dims.y, 1);

        let image_memory_barriers = [vk::ImageMemoryBarrier2 {
            src_stage_mask: vk::PipelineStageFlags2::COMPUTE_SHADER,
            src_access_mask: vk::AccessFlags2::SHADER_WRITE,
            dst_stage_mask: vk::PipelineStageFlags2::NONE,
            dst_access_mask: vk::AccessFlags2::NONE,
            old_layout: vk::ImageLayout::GENERAL,
            new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: frame.display.images.get(frame.index()).unwrap().handle,
            subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
            ..Default::default()
        }];
        cmds.pipeline_barrier(&image_memory_barriers, &[]);
    }

    fn update_uniform_buffer(&self, frame: &FrameRef, buffer: &Buffer, world: &World) {
        let ptr = buffer.get_ptr().cast::<UniformData>().as_ptr();

        let aspect = frame.display.dims.as_vec2();
        let aspect_ratio = aspect.x / aspect.y;

        let seed = rand::random();

        let forward = world.camera.rotation * glam::vec3(0.0, 0.0, 1.0);
        let up = world.camera.rotation * glam::vec3(0.0, 1.0, 0.0);

        unsafe {
            ptr.write(UniformData {
                seed,
                samples: world.settings.samples,
                bounces: world.settings.bounces,
                mode: 0,

                focal_length: world.settings.focal_length,
                aperture: world.settings.aperture,
                exposure: world.settings.exposure,
                time: 0.0,

                pos: world.camera.position.into(),

                inv_view: glam::Mat4::look_to_lh(world.camera.position.into(), forward, up).inverse(),
                inv_proj: glam::Mat4::perspective_lh(
                    f32::to_radians(world.settings.fov),
                    aspect_ratio,
                    world.settings.near,
                    world.settings.far,
                )
                .inverse(),
            })
        };
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
        log::info!("Loaded {} textures in {:?}", self.textures.len(), time);
    }

    fn load_objects(&mut self, frame: &FrameRef, objects: Vec<GpuObject>) {
        self.meshes.clear();

        let start = Instant::now();
        let mut geometries = Vec::with_capacity(objects.len());
        for (index, object) in objects.iter().enumerate() {
            // TODO: Refactor into individual functions
            // ------- Initialise Object Buffers --------------
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

            // ---------- Copy Vertices into GPU Buffer through staging buffer ------------------

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

            // ---------- Copy Indices into GPU Buffer through staging buffer ------------------

            let ptr = staging.get_ptr().cast::<u32>().as_ptr();
            unsafe { ptr.copy_from_nonoverlapping(object.indices.as_ptr(), object.indices.len()) }

            let cmds = self.command_pool.allocate();
            let region = vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: (object.indices.len() * std::mem::size_of::<u32>()) as u64,
            };

            cmds.begin();
            cmds.copy_buffer(&staging, &indices, &[region]);
            cmds.end();
            frame.context.submit(&[cmds], None, None, Some(&fence));
            fence.wait_and_reset();

            // ------------------ Copy material data -------------------------------------------
            let ptr = unsafe {
                self.material_buffer
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

            // --------- Add to geometries to build a BVH for -----------------
            let geometry = GeometryDescription {
                vertices: vertices.get_addr(),
                indices: indices.get_addr(),
                max_vertex: (object.vertices.len() - 1) as u32,
                primitives: object.indices.len().div_ceil(3) as u32,
            };

            geometries.push(geometry);

            // ------------------- Add mesh to mesh list -------------------------
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
                index: index as u32,
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
