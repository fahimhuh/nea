use crate::vulkan::{context::Context, display::Display};
use std::sync::Arc;
use winit::window::Window;

pub struct Renderer {
    context: Arc<Context>,
    display: Display,
}

impl Renderer {
    pub fn new(window: &Window) -> Self {
        let context = Arc::new(Context::new(window));
        let display = Display::new(context.clone(), window);

        Self { context, display }
    }
}
