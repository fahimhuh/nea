use super::context::Context;
use ash::vk;
use std::sync::Arc;

pub struct GeometryDescription {
    pub vertex_address: vk::DeviceAddress,
    pub indices_address: vk::DeviceAddress,

    pub max_vertex: u32,
    pub prim_count: u32,
}

struct BLASBuildInfo {
	geometry: vk::AccelerationStructureGeometryKHR,
	range: vk::AccelerationStructureBuildRangeInfoKHR
}

pub struct AccelerationStructure {
    context: Arc<Context>,
    pub handle: vk::AccelerationStructureKHR,
}

impl AccelerationStructure {
    pub fn build_blas(descs: &[GeometryDescription]) -> Vec<Self> {
        for desc in descs {
            let triangle_data = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                .vertex_format(vk::Format::R32G32B32A32_SFLOAT)
                .vertex_data(vk::DeviceOrHostAddressConstKHR {
                    device_address: desc.vertex_address,
                })
                .vertex_stride((std::mem::size_of::<f32>() * 3) as u64)
                .index_type(vk::IndexType::UINT32)
                .index_data(vk::DeviceOrHostAddressConstKHR {
                    device_address: desc.indices_address,
                })
                .max_vertex(desc.max_vertex);

            let geometry = vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                .flags(vk::GeometryFlagsKHR::OPAQUE)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    triangles: triangle_data.build(),
                });

            let build_range = vk::AccelerationStructureBuildRangeInfoKHR {
                primitive_count: desc.prim_count,
                primitive_offset: 0,
                first_vertex: 0,
                transform_offset: 0,
            };

			
        }

        todo!()
    }
}
