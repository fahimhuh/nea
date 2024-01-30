use interface::Interface;
use loader::SceneLoader;
use render::Renderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use world::World;

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

    let mut world = World::new();
    let mut renderer = Renderer::new(&window);
    let mut interface = Interface::new(&window);

    event_loop
        .run(|event, target| {
            target.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent {
                    window_id: _,
                    event,
                } => match event {
                    WindowEvent::CloseRequested => target.exit(),
                    event => interface.handle_event(&window, event),
                },

                Event::AboutToWait => {
                    world.update();
                    interface.update(&window, &mut world);
                    renderer.render(&world, &mut interface);
                }

                _ => (),
            }
        })
        .unwrap();
}
