use egui::Context;
use egui_winit::State;
use parking_lot::Once;
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

pub struct Interface {
    context: Context,
    state: State,
}

impl Interface {
    fn new(window: &Window) -> Self {
        let context = Context::default();
        let state = State::new(context.clone(), context.viewport_id(), window, None, None);

        Self { context, state }
    }
}
