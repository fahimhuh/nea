use super::{buffer::Buffer, command::CommandPool, context::Context, sync::Fence};
use ash::vk;
use std::sync::Arc;

pub struct GeometryDescription {
    vertices: vk::DeviceAddress,
    indices: vk::DeviceAddress,
    max_vertex: u32,
    primitives: u32,
}

struct BlasBuild {
    size_info: vk::AccelerationStructureBuildSizesInfoKHR,
    build_info: vk::AccelerationStructureBuildGeometryInfoKHR,
    range: vk::AccelerationStructureBuildRangeInfoKHR,
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
                .max_vertex(desc.max_vertex)
                .build();

            let geometry = vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                .flags(vk::GeometryFlagsKHR::OPAQUE)
                .geometry(vk::AccelerationStructureGeometryDataKHR { triangles })
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
                .geometries(&[geometry])
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
                build_info,
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

            build.build_info.dst_acceleration_structure = handle;
            build.build_info.scratch_data.device_address = scratch_buffer.get_addr();

            let cmds = command_pool.allocate();
            cmds.begin();
            cmds.build_acceleration_structures(&[build.build_info], &[&[build.range]]);

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
}
