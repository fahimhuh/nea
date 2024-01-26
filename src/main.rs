use assets::AssetServer;
use render::Renderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use world::World;

mod assets;
mod render;
mod vulkan;
mod world;

fn main() {
    AssetServer::init();

    let event_loop = EventLoop::new().unwrap();
    let window = Window::new(&event_loop).unwrap();

    let _world = World::new();

    let _renderer = Renderer::new(&window);

    event_loop
        .run(|event, target| {
            target.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent {
                    window_id: _,
                    event,
                } => match event {
                    WindowEvent::CloseRequested => target.exit(),
                    _ => (),
                },

                Event::AboutToWait => {}
                _ => (),
            }
        })
        .unwrap();
}
