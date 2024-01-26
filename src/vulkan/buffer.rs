use super::context::Context;
use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc};
use std::{ffi::c_void, ptr::NonNull, sync::Arc};

pub struct Buffer {
    context: Arc<Context>,
    pub handle: vk::Buffer,
    pub allocation: Allocation,
}

impl Buffer {
    pub fn new(
        context: Arc<Context>,
        size: u64,
        usage: vk::BufferUsageFlags,
        location: gpu_allocator::MemoryLocation,
        debug_name: &str,
    ) -> Self {
        let buffer_info = vk::BufferCreateInfo::builder().size(size).usage(usage);
        let handle = unsafe { context.device.create_buffer(&buffer_info, None).unwrap() };

        let requirements = unsafe { context.device.get_buffer_memory_requirements(handle) };

        let allocation = context
            .allocator
            .lock()
            .allocate(&AllocationCreateDesc {
                name: debug_name,
                requirements,
                location,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .unwrap();

        unsafe {
            context
                .device
                .bind_buffer_memory(handle, allocation.memory(), allocation.offset())
                .unwrap()
        };

        Self {
            context,
            handle,
            allocation,
        }
    }

    pub fn get_ptr(&self) -> NonNull<c_void> {
        self.allocation.mapped_ptr().unwrap()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.context
            .allocator
            .lock()
            .free(std::mem::take(&mut self.allocation))
            .unwrap();
        unsafe { self.context.device.destroy_buffer(self.handle, None) };
    }
}
