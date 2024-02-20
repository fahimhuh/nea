use super::context::Context;
use ash::vk;
use std::sync::Arc;

// A fence is a synchronization primitive that can be used to wait for a command buffer to finish executing on the GPU.
pub struct Fence {
    // The context is a handle to the Vulkan instance, device, and queue.
    pub context: Arc<Context>,
    // The handle is a handle to the Vulkan fence object.
    pub handle: vk::Fence,
}

impl Fence {
    // Creates a new fence with the specified context and signaled state.
    pub fn new(context: Arc<Context>, signaled: bool) -> Self {
        // Set the flags based on the signaled state.
        let flags = if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            vk::FenceCreateFlags::empty()
        };

        // Create the fence using the Vulkan device.
        let handle = unsafe {
            context
                .device
                .create_fence(&vk::FenceCreateInfo::builder().flags(flags), None)
                .unwrap()
        };

        // Return the new fence.
        Self { context, handle }
    }

    // Waits for the fence to be signaled and then resets it.
    pub fn wait_and_reset(&self) {
        unsafe {
            // Wait for the fence to be signaled.
            self.context
                .device
                .wait_for_fences(std::slice::from_ref(&self.handle), false, u64::MAX)
                .unwrap();

            // Reset the fence to the unsignaled state.
            self.context
                .device
                .reset_fences(std::slice::from_ref(&self.handle))
                .unwrap();
        }
    }
}

impl Drop for Fence {
    // Destroys the fence and frees its resources.
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_fence(self.handle, None) }
    }
}

// A semaphore is a synchronization primitive that can be used to synchronize operations between command buffers.
// it cannot be used to synchronize operations between the CPU and the GPU, its exclusive purpose is to synchronize operations on the GPU.
pub struct Semaphore {
    // The context is a handle to the Vulkan instance, device, and queue.
    context: Arc<Context>,
    // The handle is a handle to the Vulkan semaphore object.
    pub handle: vk::Semaphore,
}

impl Semaphore {
    // Creates a new semaphore with the specified context.
    pub fn new(context: Arc<Context>) -> Self {
        let handle = unsafe {
            context
                .device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .unwrap()
        };

        // Return the new semaphore.
        Self { context, handle }
    }
}

impl Drop for Semaphore {
    // Destroys the semaphore and frees its resources.
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_semaphore(self.handle, None) };
    }
}
