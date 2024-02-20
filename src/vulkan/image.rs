use super::context::Context;
use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc};
use std::sync::Arc;

// Represents a Vulkan image, which is a 1D, 2D, or 3D array of texels that can be used as a texture or render target.
// The image is backed by a buffer in GPU memory and can be used to store color, depth, or stencil data. This is
// seperate from buffers as GPU memory is allocated differently for images and buffers.
pub struct Image {
    // The context is a handle to the Vulkan instance, device, and queue.
    context: Arc<Context>,
    // The handle is a handle to the Vulkan image object.s
    pub handle: vk::Image,
    // The allocation is a handle to the Vulkan image memory object.
    pub allocation: Option<Allocation>,
}

impl Image {
    // Creates a new Vulkan image with the specified extent, format, usage, and debug name.
    pub fn new(
        context: Arc<Context>,
        extent: glam::UVec3,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        debug_name: &str,
    ) -> Self {
        // Get the image type based on the extent.
        let image_type = if extent.z > 1 {
            vk::ImageType::TYPE_3D
        } else {
            vk::ImageType::TYPE_2D
        };

        // Create the image using the Vulkan device.
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

        // Get the memory requirements for the image.
        let requirements = unsafe { context.device.get_image_memory_requirements(handle) };

        // Allocate memory for the image using the GPU allocator.
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

        // Bind the image to the allocated memory.
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

    // Creates a new Vulkan image from a raw handle. (For use with swapchain images.)
    pub fn from_raw(context: Arc<Context>, image: vk::Image) -> Self {
        Image {
            context,
            handle: image,
            allocation: None,
        }
    }

    // Helper function to create a default image subresource range.
    pub fn default_subresource(aspect: vk::ImageAspectFlags) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .base_mip_level(0)
            .base_array_layer(0)
            .aspect_mask(aspect)
            .level_count(1)
            .layer_count(1)
            .build()
    }

    // Helper function to create a default component mapping.
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
            // Free the image memory and destroy the image if we are the owner.
            if let Some(allocation) = std::mem::take(&mut self.allocation) {
                self.context.allocator.lock().free(allocation).unwrap();
                self.context.device.destroy_image(self.handle, None);
            }
        }
    }
}

// An image view is a view into a Vulkan image that describes how the image should be interpreted.
// It can be used to reinterpret the image's format, type, and layout, and to specify which
// subresources of the image are accessible to the view. This is useful for creating different
// views of the same image for different purposes, such as color, depth, and stencil attachments.
pub struct ImageView {
    // The context is a handle to the Vulkan instance, device, and queue.
    context: Arc<Context>,
    // The handle is a handle to the Vulkan image view object.
    pub handle: vk::ImageView,
}

impl ImageView {
    // Creates a new image view with the specified context, image, format, and subresource range.
    pub fn new(
        context: Arc<Context>,
        image: &Image,
        format: vk::Format,
        subresource: vk::ImageSubresourceRange,
    ) -> Self {
        // Create the image view using the Vulkan device.
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

        // Return the new image view.
        Self { context, handle }
    }
}

impl Drop for ImageView {
    // Destroys the image view and frees its resources.
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_image_view(self.handle, None) };
    }
}

// A sampler is a state object that contains the configuration for filtering, addressing, and
// comparing texels in a Vulkan image. It can be used to sample an image in a shader program.
pub struct Sampler {
    // The context is a handle to the Vulkan instance, device, and queue.
    context: Arc<Context>,
    // The handle is a handle to the Vulkan sampler object.
    pub handle: vk::Sampler,
}

impl Sampler {
    // Creates a new sampler with the specified context, address mode, and filter.
    pub fn new(
        context: Arc<Context>,
        address_mode: vk::SamplerAddressMode,
        filter: vk::Filter,
    ) -> Self {
        // Create the sampler using the Vulkan device.
        let create_info = vk::SamplerCreateInfo::builder()
            .address_mode_u(address_mode)
            .address_mode_v(address_mode)
            .address_mode_w(address_mode)
            .anisotropy_enable(false)
            .min_filter(filter)
            .mag_filter(filter)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .min_lod(0.0)
            .max_lod(vk::LOD_CLAMP_NONE);

        // Return the new sampler.
        let handle = unsafe { context.device.create_sampler(&create_info, None) }.unwrap();

        Self { context, handle }
    }
}

impl Drop for Sampler {
    // Destroys the sampler and frees its resources.
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_sampler(self.handle, None) }
    }
}
