use super::context::Context;
use ash::vk;
use std::{ffi::CString, sync::Arc};

pub struct Shader {
    context: Arc<Context>,
    pub handle: vk::ShaderModule,
    pub stage: vk::ShaderStageFlags,
    pub entry: CString,
}

impl Shader {
    pub fn new(
        context: Arc<Context>,
        code: &[u32],
        stage: vk::ShaderStageFlags,
        entry: &str,
    ) -> Self {
        let handle = {
            let create_info = vk::ShaderModuleCreateInfo::builder().code(code);

            unsafe {
                context
                    .device
                    .create_shader_module(&create_info, None)
                    .unwrap()
            }
        };

        let entry = CString::new(entry).unwrap();

        Self {
            context,
            handle,
            stage,
            entry,
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_shader_module(self.handle, None) }
    }
}
