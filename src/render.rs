use crate::{
    vulkan::{context::Context, display::Display},
    world::World,
};
use std::sync::Arc;
use winit::window::Window;
use self::frame::Frames;

mod frame;
mod painter;
mod raytracer;

pub struct Renderer {
    context: Arc<Context>,
    display: Arc<Display>,
    frames: Frames
}

impl Renderer {
    pub fn new(window: &Window) -> Self {
        let context = Arc::new(Context::new(window));
        let display = Display::new(context.clone(), window);
        let frames = Frames::new(context.clone(), display.clone());

        Self { context, display, frames }
    }

    pub fn render(&mut self, _world: &World) {
        let frame = self.frames.next();
        
    }
}
