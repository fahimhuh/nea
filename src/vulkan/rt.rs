use super::{buffer::Buffer, command::CommandPool, context::Context, sync::Fence};
use ash::vk::{self, Packed24_8};
use std::sync::Arc;

pub struct GeometryDescription {
    pub vertices: vk::DeviceAddress,
    pub indices: vk::DeviceAddress,
    pub max_vertex: u32,
    pub primitives: u32,
}

struct BlasBuild {
    size_info: vk::AccelerationStructureBuildSizesInfoKHR,
    geometry: vk::AccelerationStructureGeometryKHR,
    range: vk::AccelerationStructureBuildRangeInfoKHR,
}

pub struct GeometryInstance {
    pub transform: glam::Mat4,
    pub blas: vk::DeviceAddress,
}

pub struct AccelerationStructure {
    context: Arc<Context>,
    pub handle: vk::AccelerationStructureKHR,
    pub buffer: Buffer,
}

impl AccelerationStructure {
    pub fn build_bottom_levels(context: Arc<Context>, descs: &[GeometryDescription]) -> Vec<Self> {
        let mut builds = Vec::with_capacity(descs.len());
        let mut scratch_size = 0;

        for desc in descs {
            let triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                .vertex_format(vk::Format::R32G32B32A32_SFLOAT)
                .vertex_data(vk::DeviceOrHostAddressConstKHR {
                    device_address: desc.vertices,
                })
                .vertex_stride((std::mem::size_of::<f32>() * 3) as u64)
                .index_type(vk::IndexType::UINT32)
                .index_data(vk::DeviceOrHostAddressConstKHR {
                    device_address: desc.indices,
                })
                .max_vertex(desc.max_vertex);

            let geometry = vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                .flags(vk::GeometryFlagsKHR::OPAQUE)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    triangles: *triangles,
                })
                .build();

            let range = vk::AccelerationStructureBuildRangeInfoKHR::builder()
                .first_vertex(0)
                .primitive_count(desc.primitives)
                .primitive_offset(0)
                .transform_offset(0)
                .build();

            let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
                .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
                .geometries(std::slice::from_ref(&geometry))
                .build();

            let size_info = unsafe {
                context
                    .acceleration_structures
                    .get_acceleration_structure_build_sizes(
                        vk::AccelerationStructureBuildTypeKHR::HOST,
                        &build_info,
                        &[desc.primitives],
                    )
            };

            let build = BlasBuild {
                size_info,
                geometry,
                range,
            };

            builds.push(build);
            scratch_size = scratch_size.max(size_info.build_scratch_size);
        }

        let scratch_buffer = Buffer::new(
            context.clone(),
            scratch_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
            gpu_allocator::MemoryLocation::GpuOnly,
            "BLAS build scratch buffer",
        );

        let command_pool = CommandPool::new(context.clone(), context.queue_family);
        let fence = Fence::new(context.clone(), false);

        let mut acceleration_structures = Vec::with_capacity(builds.len());

        for build in &mut builds {
            let mut build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
                .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
                .geometries(std::slice::from_ref(&build.geometry))
                .build();

            let buffer = Buffer::new(
                context.clone(),
                build.size_info.acceleration_structure_size,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                gpu_allocator::MemoryLocation::GpuOnly,
                "BLAS Storage",
            );

            let create_info = vk::AccelerationStructureCreateInfoKHR::builder()
                .buffer(buffer.handle)
                .offset(0)
                .size(build.size_info.acceleration_structure_size)
                .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL);

            let handle = unsafe {
                context
                    .acceleration_structures
                    .create_acceleration_structure(&create_info, None)
                    .unwrap()
            };

            build_info.dst_acceleration_structure = handle;
            build_info.scratch_data.device_address = scratch_buffer.get_addr();

            let cmds = command_pool.allocate();
            cmds.begin();
            cmds.build_acceleration_structures(
                std::slice::from_ref(&build_info),
                &[std::slice::from_ref(&build.range)],
            );

            let barrier = vk::MemoryBarrier2 {
                src_stage_mask: vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                src_access_mask: vk::AccessFlags2::ACCELERATION_STRUCTURE_WRITE_KHR,
                dst_stage_mask: vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                dst_access_mask: vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR,
                ..Default::default()
            };

            cmds.pipeline_barrier(&[], &[barrier]);
            cmds.end();

            context.submit(&[cmds], None, None, Some(&fence));
            fence.wait_and_reset();

            let acceleration_structure = AccelerationStructure {
                context: context.clone(),
                handle,
                buffer,
            };

            acceleration_structures.push(acceleration_structure)
        }

        acceleration_structures
    }

    pub fn build_top_level(context: Arc<Context>, objects: &[GeometryInstance]) -> Self {
        let mut instances = Vec::with_capacity(objects.len());
        let command_pool = CommandPool::new(context.clone(), context.queue_family);
        for object in objects {
            let matrix: [f32; 12] = object
                .transform
                .transpose()
                .to_cols_array()
                .split_at(12)
                .0
                .try_into()
                .unwrap();

            let transform = vk::TransformMatrixKHR { matrix };

            let instance = vk::AccelerationStructureInstanceKHR {
                transform,
                instance_custom_index_and_mask: Packed24_8::new(0, 0xFF),
                instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(0, 0),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: object.blas,
                },
            };

            instances.push(instance);
        }

        let instance_buffer = Buffer::new(
            context.clone(),
            (std::mem::size_of::<vk::AccelerationStructureInstanceKHR>() * instances.len()) as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            gpu_allocator::MemoryLocation::CpuToGpu,
            "TLAS Instance Buffer",
        );

        unsafe {
            let ptr = instance_buffer
                .get_ptr()
                .cast::<vk::AccelerationStructureInstanceKHR>()
                .as_ptr();
            ptr.copy_from_nonoverlapping(instances.as_ptr(), instances.len());
        }

        let geometry_instances = vk::AccelerationStructureGeometryInstancesDataKHR::builder()
            .data(vk::DeviceOrHostAddressConstKHR {
                device_address: instance_buffer.get_addr(),
            })
            .build();

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: geometry_instances,
            })
            .build();

        let mut build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(std::slice::from_ref(&geometry))
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .build();

        let sizes = unsafe {
            context
                .acceleration_structures
                .get_acceleration_structure_build_sizes(
                    vk::AccelerationStructureBuildTypeKHR::DEVICE,
                    &build_info,
                    &[instances.len() as u32],
                )
        };

        let buffer = Buffer::new(
            context.clone(),
            sizes.acceleration_structure_size,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            gpu_allocator::MemoryLocation::GpuOnly,
            "BLAS Storage",
        );

        let create_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .buffer(buffer.handle)
            .offset(0)
            .size(sizes.acceleration_structure_size)
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL);

        let handle = unsafe {
            context
                .acceleration_structures
                .create_acceleration_structure(&create_info, None)
                .unwrap()
        };

        let scratch_buffer = Buffer::new(
            context.clone(),
            sizes.build_scratch_size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            gpu_allocator::MemoryLocation::GpuOnly,
            "TLAS Scratch Buffer",
        );

        build_info.dst_acceleration_structure = handle;
        build_info.scratch_data.device_address = scratch_buffer.get_addr();

        let range = vk::AccelerationStructureBuildRangeInfoKHR {
            primitive_count: instances.len() as u32,
            primitive_offset: 0,
            first_vertex: 0,
            transform_offset: 0,
        };

        let fence = Fence::new(context.clone(), false);
        let cmds = command_pool.allocate();
        cmds.begin();
        cmds.build_acceleration_structures(
            std::slice::from_ref(&build_info),
            &[std::slice::from_ref(&range)],
        );
        cmds.end();
        context.submit(&[cmds], None, None, Some(&fence));
        fence.wait_and_reset();

        Self {
            context,
            handle,
            buffer,
        }
    }

    pub fn get_addr(&self) -> vk::DeviceAddress {
        let info = vk::AccelerationStructureDeviceAddressInfoKHR::builder()
            .acceleration_structure(self.handle)
            .build();
        unsafe {
            self.context
                .acceleration_structures
                .get_acceleration_structure_device_address(&info)
        }
    }
}

impl Drop for AccelerationStructure {
    fn drop(&mut self) {
        unsafe {
            self.context
                .acceleration_structures
                .destroy_acceleration_structure(self.handle, None)
        };
    }
}
