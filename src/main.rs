use assets::AssetServer;
use render::Renderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use world::World;

mod assets;
mod interface;
mod render;
mod vulkan;
mod world;

fn main() {
    AssetServer::init();

    let event_loop = EventLoop::new().unwrap();
    let window = Window::new(&event_loop).unwrap();
    window.set_resizable(false);

    let mut world = World::new();

    let mut renderer = Renderer::new(&window);

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

                Event::AboutToWait => {
                    AssetServer::update();

                    world.update();
                    renderer.render(&world);
                }

                _ => (),
            }
        })
        .unwrap();
}
