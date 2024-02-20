use self::{frame::Frames, painter::InterfacePainter, raytracer::Raytracer};
use crate::{
    interface::Interface,
    vulkan::{context::Context, display::Display},
    world::World,
};
use std::sync::Arc;
use winit::window::Window;

mod frame;
mod painter;
mod raytracer;

// The renderer is the struct that is responsible for rendering the world and interface
// It contains the Vulkan API context, along with the Vulkan resources required to render
// the world and interface, which are contained into their own structs.
// Additonally it includes an abstraction over the window display, the `Frames` which is used
// to seperate the work of rendering between frames.
pub struct Renderer {
    // The Vulkan API context
    context: Arc<Context>,
    // The window display (which contains the Vulkan Swapchain)
    display: Arc<Display>,
    // An abstraction over the display that allows us to present and render frames
    frames: Frames,

    // The raytracer is the struct that is responsible for rendering the world
    // and contains the Vulkan resources required to do so
    raytracer: Raytracer,

    // The interface painter is the struct that is responsible for rendering the interface
    // and contains the Vulkan resources required to do so.
    painter: InterfacePainter,
}

impl Renderer {
    // Create a new renderer with the given window
    pub fn new(window: &Window) -> Self {
        // Create the Vulkan API context
        let context = Arc::new(Context::new(window));
        // Create the window display
        let display = Display::new(context.clone(), window);
        // Create the frames abstraction
        let frames = Frames::new(context.clone(), display.clone());

        // Initialise the raytracer resources
        let raytracer = Raytracer::new(context.clone());
        // Intialise the interface painter resources
        let painter = InterfacePainter::new(context.clone(), &display);

        // Return the renderer object
        Self {
            context,
            display,
            frames,

            raytracer,
            painter,
        }
    }

    // Render the world and interface
    pub fn render(&mut self, world: &mut World, interface: &mut Interface) {
        // Get the next frame from the frames abstraction
        let mut frame = self.frames.next();

        // Allocate a command list from the frame
        let cmds = frame.allocate_command_list();

        // Run the raytracer and interface painter
        cmds.begin();
        self.raytracer.run(&cmds, &frame, world);
        self.painter.draw(&cmds, &frame, interface);
        cmds.end();

        // Submit the commands to the GPU and show the output onto the screen.
        frame.submit(&[cmds]);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // Ensure that the GPU is not using any resources before we drop them
        self.context.wait_idle()
    }
}
