use super::context::Context;
use ash::vk;
use std::sync::Arc;

pub struct Fence {
    pub context: Arc<Context>,
    pub handle: vk::Fence,
}

impl Fence {
    pub fn new(context: Arc<Context>, signaled: bool) -> Self {
        let flags = if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            vk::FenceCreateFlags::empty()
        };
        let handle = unsafe {
            context
                .device
                .create_fence(&vk::FenceCreateInfo::builder().flags(flags), None)
                .unwrap()
        };

        Self { context, handle }
    }

    pub fn wait_and_reset(&self) {
        unsafe {
            self.context
                .device
                .wait_for_fences(std::slice::from_ref(&self.handle), false, u64::MAX)
                .unwrap();
            self.context
                .device
                .reset_fences(std::slice::from_ref(&self.handle))
                .unwrap();
        }
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_fence(self.handle, None) }
    }
}

pub struct Semaphore {
    context: Arc<Context>,
    pub handle: vk::Semaphore,
}

impl Semaphore {
    pub fn new(context: Arc<Context>) -> Self {
        let handle = unsafe {
            context
                .device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .unwrap()
        };

        Self { context, handle }
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_semaphore(self.handle, None) };
    }
}
