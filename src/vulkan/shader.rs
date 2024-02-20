use super::context::Context;
use ash::vk;
use std::{ffi::CString, sync::Arc};

/// Represents a Vulkan shader module, which is a compiled shader program that can be executed on the GPU.
pub struct Shader {
    /// The context is a handle to the Vulkan instance, device, and queue.
    context: Arc<Context>,
    /// The handle is a handle to the Vulkan shader module object.
    pub handle: vk::ShaderModule,
    /// The stage is the stage of a Vulkan pipeline that the shader module is used in.
    pub stage: vk::ShaderStageFlags,
    /// The entry is the name of the entry point function in the shader module.
    pub entry: CString,
}

impl Shader {
    /// Creates a new shader module with the specified context, code, stage, and entry point.
    pub fn new(
        context: Arc<Context>,
        code: &[u32],
        stage: vk::ShaderStageFlags,
        entry: &str,
    ) -> Self {
        // Create the shader module using the Vulkan device.
        let handle = {
            let create_info = vk::ShaderModuleCreateInfo::builder().code(code);

            unsafe {
                context
                    .device
                    .create_shader_module(&create_info, None)
                    .unwrap()
            }
        };

        // Convert the entry point name to a C (null-terminated) string.
        let entry = CString::new(entry).unwrap();

        // Return the new shader module.
        Self {
            context,
            handle,
            stage,
            entry,
        }
    }
}

impl Drop for Shader {
    // Destroys the shader module and frees its resources.
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_shader_module(self.handle, None) }
    }
}
