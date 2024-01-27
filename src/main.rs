use loader::SceneLoader;
use render::Renderer;
use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::PhysicalKey,
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

    event_loop
        .run(|event, target| {
            target.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent {
                    window_id: _,
                    event,
                } => match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.physical_key == PhysicalKey::Code(winit::keyboard::KeyCode::KeyR)
                            && matches!(event.state, ElementState::Released)
                        {
                            SceneLoader::request_load();
                        }
                    }
                    _ => (),
                },

                Event::AboutToWait => {
                    world.update();
                    renderer.render(&world);
                }

                _ => (),
            }
        })
        .unwrap();
}
