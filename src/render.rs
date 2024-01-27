use self::{frame::Frames, painter::InterfacePainter, raytracer::Raytracer};
use crate::{
    interface::Interface, vulkan::{context::Context, display::Display}, world::World
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
    painter: InterfacePainter
}

impl Renderer {
    pub fn new(window: &Window) -> Self {
        let context = Arc::new(Context::new(window));
        let display = Display::new(context.clone(), window);
        let frames = Frames::new(context.clone(), display.clone());

        let raytracer = Raytracer::new();
        let painter = InterfacePainter::new(context.clone(), &display);

        Self {
            context,
            display,
            frames,

            raytracer,
            painter
        }
    }

    pub fn render(&mut self, _world: &World, interface: &mut Interface) {
        let mut frame = self.frames.next();
        let cmds = frame.allocate_command_list();
        self.painter.draw(&cmds, &frame, interface);
        
        frame.submit(&[]);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.context.wait_idle()
    }
}
