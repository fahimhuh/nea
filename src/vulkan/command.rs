use std::sync::Arc;
use ash::vk;
use super::{
    context::Context,
};

pub struct CommandPool {
    context: Arc<Context>,
    pub handle: vk::CommandPool,
}

impl CommandPool {
    pub fn new(context: Arc<Context>, queue_family: u32) -> Self {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family)
            .flags(vk::CommandPoolCreateFlags::TRANSIENT);
        let handle = unsafe {
            context
                .device
                .create_command_pool(&create_info, None)
                .unwrap()
        };
        Self { context, handle }
    }

    pub fn reset(&self) {
        unsafe {
            self.context
                .device
                .reset_command_pool(self.handle, vk::CommandPoolResetFlags::empty())
                .unwrap();
        }
    }

    pub fn allocate(&self) -> CommandList {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.handle);

        let handle = unsafe { self.context.device.allocate_command_buffers(&allocate_info) }
            .unwrap()
            .remove(0);

        CommandList {
            context: self.context.clone(),
            handle,
        }
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_command_pool(self.handle, None) }
    }
}

pub struct CommandList {
    pub context: Arc<Context>,
    pub handle: vk::CommandBuffer,
}
