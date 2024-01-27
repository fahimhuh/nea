use self::{frame::Frames, raytracer::Raytracer};
use crate::{
    vulkan::{context::Context, display::Display},
    world::World,
};
use std::sync::Arc;
use winit::window::Window;

mod frame;
mod painter;
mod raytracer;

pub struct Renderer {
    context: Arc<Context>,
    display: Arc<Display>,
    frames: Frames,

    raytracer: Raytracer,
}

impl Renderer {
    pub fn new(window: &Window) -> Self {
        let context = Arc::new(Context::new(window));
        let display = Display::new(context.clone(), window);
        let frames = Frames::new(context.clone(), display.clone());

        let raytracer = Raytracer::new();

        Self {
            context,
            display,
            frames,

            raytracer,
        }
    }

    pub fn render(&mut self, _world: &World) {
        let mut frame = self.frames.next();
        self.raytracer.run();
        frame.submit(&[]);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.context.wait_idle()
    }
}
