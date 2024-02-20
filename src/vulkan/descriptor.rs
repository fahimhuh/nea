use super::{buffer::Buffer, context::Context, image::ImageView, rt::AccelerationStructure};
use ash::vk;
use std::{ffi::c_void, sync::Arc};

// This structure contains the information needed to write a reference to an image to a descriptor set.
pub struct DescriptorImageWrite<'a> {
    // The image view to reference.
    pub image_view: &'a ImageView,
    // An optional sampler to use with the image.
    pub sampler: Option<vk::Sampler>,
    // The kind of image to reference.
    pub image_kind: vk::DescriptorType,
    // The memory layout of the image.
    pub layout: vk::ImageLayout,
    // Where you want to bind the image.
    pub binding: u32,
}

// This structure contains the information needed to write a reference to a buffer to a descriptor set.
pub struct DescriptorBufferWrite<'a> {
    // The kind of buffer to reference.
    pub buffer_kind: vk::DescriptorType,
    // The buffer to reference.
    pub buffer: &'a Buffer,
    // The memory range of the buffer.
    pub range: u64,
    // Where you want to bind the buffer.
    pub binding: u32,
}

// This structure contains the information needed to write a reference to a top-level acceleration structure to a descriptor set.
pub struct DescriptorTLASWrite<'a> {
    // The acceleration structure to reference.
    pub reference: &'a AccelerationStructure,
    // Where you want to bind the acceleration structure.
    pub binding: u32,
}

// A descriptor set is a collection of references to images, buffers, and acceleration structures.
// It is used to bind resources to shaders.
pub struct DescriptorSet {
    // The context that the descriptor set was created with.
    context: Arc<Context>,
    // The handle to the descriptor set.
    pub handle: vk::DescriptorSet,
}

impl DescriptorSet {
    // Writes references to images, buffers to the descriptor set.
    pub fn write(&self, images: &[DescriptorImageWrite], buffers: &[DescriptorBufferWrite]) {
        for image in images {
            // Create a descriptor image info structure.
            let image_info = vk::DescriptorImageInfo {
                sampler: image.sampler.unwrap_or(vk::Sampler::null()),
                image_view: image.image_view.handle,
                image_layout: image.layout,
            };

            let write = vk::WriteDescriptorSet::builder()
                .descriptor_type(image.image_kind)
                .dst_array_element(0)
                .dst_binding(image.binding)
                .dst_set(self.handle)
                .image_info(std::slice::from_ref(&image_info))
                .build();

            unsafe {
                // Update the descriptor set with the image reference.
                self.context
                    .device
                    .update_descriptor_sets(std::slice::from_ref(&write), &[])
            };
        }

        for buffer in buffers {
            // Create a descriptor buffer info structure.
            let buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(buffer.buffer.handle)
                .offset(0)
                .range(buffer.range)
                .build();

            let write = vk::WriteDescriptorSet::builder()
                .descriptor_type(buffer.buffer_kind)
                .dst_array_element(0)
                .dst_binding(buffer.binding)
                .dst_set(self.handle)
                .buffer_info(std::slice::from_ref(&buffer_info))
                .build();

            unsafe {
                // Update the descriptor set with the buffer reference.
                self.context
                    .device
                    .update_descriptor_sets(std::slice::from_ref(&write), &[])
            };
        }
    }

    // Writes a reference to a top-level acceleration structure to the descriptor set.
    pub fn write_tlas(&self, tlas: DescriptorTLASWrite) {
        // Create a pointer to the acceleration structure handle.
        let handles = [tlas.reference.handle];

        let tlas_write = vk::WriteDescriptorSetAccelerationStructureKHR {
            acceleration_structure_count: 1,
            p_acceleration_structures: handles.as_ptr(),
            ..Default::default()
        };

        let ptr =
            (&tlas_write as *const vk::WriteDescriptorSetAccelerationStructureKHR).cast::<c_void>();

        // Create a descriptor write structure.
        let write = vk::WriteDescriptorSet {
            p_next: ptr,
            dst_set: self.handle,
            dst_binding: tlas.binding,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
            ..Default::default()
        };

        unsafe {
            // Update the descriptor set with the acceleration structure reference.
            self.context
                .device
                .update_descriptor_sets(std::slice::from_ref(&write), &[])
        };
    }
}

// A descriptor pool is a collection of resources that can be used to create descriptor sets.
// It is used to allocate and free descriptor sets.
pub struct DescriptorPool {
    context: Arc<Context>,
    pub handle: vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(context: Arc<Context>) -> Self {
        // Define the types of resources that the descriptor pool will contain.
        let storage_images = vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(100)
            .build();

        let uniform_buffers = vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(100)
            .build();

        let sampled_images = vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(100)
            .build();

        let tlasses = vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(100)
            .build();

        let sizes = [storage_images, uniform_buffers, sampled_images, tlasses];

        // Create the descriptor pool.
        let create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&sizes)
            .max_sets(100);

        let handle = unsafe {
            context
                .device
                .create_descriptor_pool(&create_info, None)
                .unwrap()
        };

        // Return the descriptor pool.
        Self { context, handle }
    }

    // Allocates a collection of descriptor sets from the descriptor pool.
    pub fn allocate(
        &self,
        context: &Context,
        layout: &DescriptorSetLayout,
        count: usize,
    ) -> Vec<DescriptorSet> {
        // Create a collection of descriptor set layouts.
        let set_layouts = std::iter::repeat(layout.handle)
            .take(count)
            .collect::<Vec<_>>();

        // Allocate the descriptor sets.
        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.handle)
            .set_layouts(&set_layouts);

        let sets = unsafe {
            context
                .device
                .allocate_descriptor_sets(&allocate_info)
                .unwrap()
                .into_iter()
                .map(|handle| DescriptorSet {
                    handle,
                    context: self.context.clone(),
                })
                .collect::<Vec<DescriptorSet>>()
        };

        // Return the descriptor sets.
        sets
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        // Destroy the descriptor pool
        unsafe {
            self.context
                .device
                .destroy_descriptor_pool(self.handle, None)
        };
    }
}

// A descriptor binding is a reference to a resource in a shader.
pub struct DescriptorBinding {
    pub binding: u32,
    pub count: u32,
    pub kind: vk::DescriptorType,
    pub stage: vk::ShaderStageFlags,
}

// A descriptor set layout is a collection of descriptor bindings and describes the types of resources that can be referenced in a descriptor set.
pub struct DescriptorSetLayout {
    // The context that the descriptor set layout was created with.
    context: Arc<Context>,
    // The handle to the descriptor set layout.
    pub handle: vk::DescriptorSetLayout,
}

impl DescriptorSetLayout {
    // Creates a new descriptor set layout.
    pub fn new(context: Arc<Context>, bindings: Vec<DescriptorBinding>) -> Self {
        // Convert the bindings into a collection of Vulkan descriptor set layout bindings.
        let bindings = bindings
            .into_iter()
            .map(|db| {
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(db.binding)
                    .descriptor_count(db.count)
                    .descriptor_type(db.kind)
                    .stage_flags(db.stage)
                    .build()
            })
            .collect::<Vec<_>>();

        // Create the descriptor set layout.
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

        let handle = unsafe {
            context
                .device
                .create_descriptor_set_layout(&create_info, None)
                .unwrap()
        };

        // Return the descriptor set layout.
        Self { context, handle }
    }
}

impl Drop for DescriptorSetLayout {
    // Destroys the descriptor set layout.
    fn drop(&mut self) {
        unsafe {
            self.context
                .device
                .destroy_descriptor_set_layout(self.handle, None)
        }
    }
}
