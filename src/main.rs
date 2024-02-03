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
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    SceneLoader::init();
    let event_loop = EventLoop::new().unwrap();
    let window = Window::new(&event_loop).unwrap();
    window.set_resizable(false);
    let inputs = Inputs::new();
    let mut renderer = Renderer::new(&window);
    let mut world = World::new();
    let mut interface = Interface::new(&window);

    event_loop
        .run(|event, target| {
            target.set_control_flow(ControlFlow::Poll);

            match event {
                
                // Handle device events
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => target.exit(),
                    event => interface.handle_event(&window, event, &inputs),
                },

                Event::DeviceEvent { event, .. } => {
                    inputs.broadcaster.try_send(Input::from_device_event(event));
                }

                // Main application loop
                Event::AboutToWait => {
                    world.update(&inputs);
                    interface.update(&window, &mut world);
                    renderer.render(&mut world, &mut interface);
                }

                _ => (),
            }
        })
        .unwrap();
}
