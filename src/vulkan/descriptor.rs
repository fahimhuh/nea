use super::{buffer::Buffer, context::Context, image::ImageView, rt::AccelerationStructure};
use ash::vk;
use std::{ffi::c_void, sync::Arc};

pub struct DescriptorImageWrite<'a> {
    pub image_view: &'a ImageView,
    pub sampler: Option<vk::Sampler>,
    pub image_kind: vk::DescriptorType,
    pub layout: vk::ImageLayout,
    pub binding: u32,
}

pub struct DescriptorBufferWrite<'a> {
    pub buffer_kind: vk::DescriptorType,
    pub buffer: &'a Buffer,
    pub range: u64,
    pub binding: u32,
}

pub struct DescriptorTLASWrite<'a> {
    pub reference: &'a AccelerationStructure,
    pub binding: u32,
}

pub struct DescriptorSet {
    context: Arc<Context>,
    pub handle: vk::DescriptorSet,
}

impl DescriptorSet {
    pub fn write(&self, images: &[DescriptorImageWrite], buffers: &[DescriptorBufferWrite]) {
        for image in images {
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
                self.context
                    .device
                    .update_descriptor_sets(std::slice::from_ref(&write), &[])
            };
        }

        for buffer in buffers {
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
                self.context
                    .device
                    .update_descriptor_sets(std::slice::from_ref(&write), &[])
            };
        }
    }

    pub fn write_tlas(&self, tlas: DescriptorTLASWrite) {
        let handles = [tlas.reference.handle];
        
        let tlas_write = vk::WriteDescriptorSetAccelerationStructureKHR {
            acceleration_structure_count: 1,
            p_acceleration_structures: handles.as_ptr(),
            ..Default::default()
        };

        let ptr = (&tlas_write as *const vk::WriteDescriptorSetAccelerationStructureKHR).cast::<c_void>();
        
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
            self.context
                .device
                .update_descriptor_sets(std::slice::from_ref(&write), &[])
        };
    }
}

pub struct DescriptorPool {
    context: Arc<Context>,
    pub handle: vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(context: Arc<Context>) -> Self {
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

        let create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&sizes)
            .max_sets(100);

        let handle = unsafe {
            context
                .device
                .create_descriptor_pool(&create_info, None)
                .unwrap()
        };

        Self { context, handle }
    }

    pub fn allocate(
        &self,
        context: &Context,
        layout: &DescriptorSetLayout,
        count: usize,
    ) -> Vec<DescriptorSet> {
        let set_layouts = std::iter::repeat(layout.handle)
            .take(count)
            .collect::<Vec<_>>();

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

        sets
    }

    pub fn free(&self, context: &Context, set: vk::DescriptorSet) {
        unsafe {
            context
                .device
                .free_descriptor_sets(self.handle, &[set])
                .unwrap();
        }
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.context
                .device
                .destroy_descriptor_pool(self.handle, None)
        };
    }
}

pub struct DescriptorBinding {
    pub binding: u32,
    pub count: u32,
    pub kind: vk::DescriptorType,
    pub stage: vk::ShaderStageFlags,
}

pub struct DescriptorSetLayout {
    context: Arc<Context>,
    pub handle: vk::DescriptorSetLayout,
}

impl DescriptorSetLayout {
    pub fn new(context: Arc<Context>, bindings: Vec<DescriptorBinding>) -> Self {
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

        let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

        let handle = unsafe {
            context
                .device
                .create_descriptor_set_layout(&create_info, None)
                .unwrap()
        };

        Self { context, handle }
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.context
                .device
                .destroy_descriptor_set_layout(self.handle, None)
        }
    }
}
