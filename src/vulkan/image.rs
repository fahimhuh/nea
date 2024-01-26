use std::sync::Arc;

use super::context::Context;
use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc};

pub struct Image {
    context: Arc<Context>,
    pub handle: vk::Image,
    pub allocation: Option<Allocation>,
}

impl Image {
    pub fn new(
        context: Arc<Context>,
        extent: glam::UVec3,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        debug_name: &str,
    ) -> Self {
        let image_type = if extent.z > 1 {
            vk::ImageType::TYPE_3D
        } else {
            vk::ImageType::TYPE_2D
        };
        let create_info = vk::ImageCreateInfo {
            flags: vk::ImageCreateFlags::empty(),
            image_type,
            format,
            extent: vk::Extent3D {
                width: extent.x,
                height: extent.y,
                depth: extent.z,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            ..Default::default()
        };

        let handle = unsafe { context.device.create_image(&create_info, None).unwrap() };
        let requirements = unsafe { context.device.get_image_memory_requirements(handle) };

        let allocation = context
            .allocator
            .lock()
            .allocate(&AllocationCreateDesc {
                name: debug_name,
                requirements,
                location: gpu_allocator::MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .unwrap();

        unsafe {
            context
                .device
                .bind_image_memory(handle, allocation.memory(), allocation.offset())
                .unwrap();
        }

        Self {
            context,
            handle,
            allocation: Some(allocation),
        }
    }

    pub fn from_raw(context: Arc<Context>, image: vk::Image) -> Self {
        Image {
            context,
            handle: image,
            allocation: None,
        }
    }

    pub fn default_subresource(aspect: vk::ImageAspectFlags) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .base_mip_level(0)
            .base_array_layer(0)
            .aspect_mask(aspect)
            .level_count(1)
            .layer_count(1)
            .build()
    }

    pub fn default_component_mapping() -> vk::ComponentMapping {
        vk::ComponentMapping {
            r: vk::ComponentSwizzle::R,
            g: vk::ComponentSwizzle::G,
            b: vk::ComponentSwizzle::B,
            a: vk::ComponentSwizzle::A,
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            if let Some(allocation) = std::mem::take(&mut self.allocation) {
                self.context.allocator.lock().free(allocation).unwrap();
                self.context.device.destroy_image(self.handle, None);
            }
        }
    }
}

pub struct ImageView {
    context: Arc<Context>,
    pub handle: vk::ImageView,
}

impl ImageView {
    pub fn new(
        context: Arc<Context>,
        image: &Image,
        format: vk::Format,
        subresource: vk::ImageSubresourceRange,
    ) -> Self {
        let create_info = vk::ImageViewCreateInfo::builder()
            .image(image.handle)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(subresource)
            .components(Image::default_component_mapping());

        let handle = unsafe {
            context
                .device
                .create_image_view(&create_info, None)
                .unwrap()
        };

        Self { context, handle }
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_image_view(self.handle, None) };
    }
}
