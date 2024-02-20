use super::context::Context;
use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc};
use std::{ffi::c_void, ptr::NonNull, sync::Arc};

/// Represents a Vulkan buffer, which is a fixed-size block of memory used to store data on the GPU's VRAM.
pub struct Buffer {
    context: Arc<Context>,
    pub handle: vk::Buffer,
    pub allocation: Allocation,
}

impl Buffer {
    /// Creates a new Vulkan buffer with the specified size, usage, memory location, and debug name.
    pub fn new(
        context: Arc<Context>,
        size: u64,
        usage: vk::BufferUsageFlags,
        location: gpu_allocator::MemoryLocation,
        debug_name: &str,
    ) -> Self {
        // Create the buffer using the Vulkan device.
        let buffer_info = vk::BufferCreateInfo::builder().size(size).usage(usage);
        let handle = unsafe { context.device.create_buffer(&buffer_info, None).unwrap() };

        // Get the memory requirements for the buffer.
        let requirements = unsafe { context.device.get_buffer_memory_requirements(handle) };

        // Allocate memory for the buffer using the GPU allocator.
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

        // Bind the buffer to the allocated memory.
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

    /// Returns a non-null pointer to the mapped memory of the buffer.
    pub fn get_ptr(&self) -> NonNull<c_void> {
        self.allocation.mapped_ptr().unwrap()
    }

    /// Returns the device address of the buffer.
    pub fn get_addr(&self) -> vk::DeviceAddress {
        unsafe {
            self.context
                .device
                .get_buffer_device_address(&vk::BufferDeviceAddressInfo {
                    buffer: self.handle,
                    ..Default::default()
                })
        }
    }
}

impl Drop for Buffer {
    /// Frees the allocated memory and destroys the Vulkan buffer when the Buffer object is dropped.
    fn drop(&mut self) {
        self.context
            .allocator
            .lock()
            .free(std::mem::take(&mut self.allocation))
            .unwrap();
        unsafe { self.context.device.destroy_buffer(self.handle, None) };
    }
}
