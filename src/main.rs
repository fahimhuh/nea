use input::{Input, Inputs};
use interface::Interface;
use loader::SceneLoader;
use render::Renderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use world::World;

mod input;
mod interface;
mod loader;
mod render;
mod vulkan;
mod world;

fn main() {
    // Initialise the logger library, which will output timestamped logs to the console
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Initilialise the eventloop, which is the construct that is used to recieve
    // and handle events from the window
    let event_loop = EventLoop::new().unwrap();
    // And create a window to recieve events from
    let window = Window::new(&event_loop).unwrap();
    // We set the window to be be non-resizable as this significantly complicates the rendering process
    // and is not necessary for this project. Recall how in most video-games, the window is not resizable
    // rather there is a dropdown menu with a fixed number of resolutions to choose from.
    window.set_resizable(false);

    // Create the input queue, which will be used to store input events
    let inputs = Inputs::new();

    // Initialise the world, which is where the data about the camera, rendering settings and objects are stored (On the CPU side of things)
    let mut world = World::new();

    // Initialise the interface, which is the construct that integrates this program with the egui library
    // (However, it is not responsible for the rendering of the interface, that is done by the Renderer struct)
    let mut interface = Interface::new(&window);

    // Initialise the renderer, which is the construct that is responsible for rendering the world and interface
    let mut renderer = Renderer::new(&window);

    // Initialise the scene loader, which allows us to spawn threads to load scenes in the background
    SceneLoader::init();

    // Run the event loop
    event_loop
        .run(|event, target| {
            // Tell the window system we want to be called again as soon as possible
            target.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => match event {
                    // Close when the user clicks the close button
                    WindowEvent::CloseRequested => target.exit(),

                    // Pass all other window events to the interface
                    event => interface.handle_event(&window, event, &inputs),
                },

                Event::DeviceEvent { event, .. } => {
                    // Pass all device events to the input queue
                    let _ = inputs.broadcaster.try_send(Input::from_device_event(event));
                }

                // Main application loop
                Event::AboutToWait => {
                    // Update the positions of the camera given the inputs from the queue
                    world.update(&inputs);

                    // Update the interface, which can also update the properties within the world
                    interface.update(&window, &mut world);

                    // And render both the world and the interface
                    renderer.render(&mut world, &mut interface);
                }

                _ => (),
            }
        })
        .unwrap();
}
