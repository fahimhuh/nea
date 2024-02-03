use crate::{
    input::{Input, Inputs},
    loader::SceneLoader,
    world::World,
};
use crossbeam_channel::Sender;
use winit::{event::WindowEvent, window::Window};

pub struct Interface {
    interface_context: egui::Context,
    window_integration: egui_winit::State,

    last_output: egui::FullOutput,
}

impl Interface {
    pub fn new(window: &Window) -> Self {
        let interface_context = egui::Context::default();
        let window_integration = egui_winit::State::new(
            interface_context.clone(),
            interface_context.viewport_id(),
            &window,
            None,
            None,
        );

        Self {
            interface_context,
            window_integration,
            last_output: egui::FullOutput::default(),
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: WindowEvent, inputs: &Inputs) {
        let response = self.window_integration.on_window_event(window, &event);
        if !response.consumed {
            inputs.broadcaster.try_send(Input::from_window_event(event));
        }
    }

    pub fn update(&mut self, window: &Window, world: &mut World) {
        let raw_input = self.window_integration.take_egui_input(window);
        self.interface_context.begin_frame(raw_input);

        self.scene_ui();
        self.camera_ui(world);

        let output = self.interface_context.end_frame();
        self.window_integration
            .handle_platform_output(window, output.platform_output.clone());

        self.last_output = output;
    }

    pub fn context(&self) -> &egui::Context {
        &self.interface_context
    }

    pub fn take_last_output(&mut self) -> egui::FullOutput {
        std::mem::take(&mut self.last_output)
    }

    pub fn camera_ui(&mut self, world: &mut World) {
        egui::Window::new("Camera").show(&self.context(), |ui| {
            // TODO: ===================== REPLACE BELOW WITH ACTUAL CAMERA INPUTS =========================

            // TODO: ===================== REPLACE AVOVE WITH ACTUAL CAMERA INPUTS =========================

            egui::Grid::new("Camera UI")
                .striped(true)
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Y FOV: ");
                    ui.add(egui::DragValue::new(&mut world.settings.fov));
                    ui.end_row();

                    ui.label("Sample count: ");
                    ui.add(egui::DragValue::new(&mut world.settings.samples));
                    ui.end_row();

                    ui.label("Bounces simulated: ");
                    ui.add(egui::DragValue::new(&mut world.settings.bounces));
                    ui.end_row();
                });
        });
    }

    pub fn scene_ui(&mut self) {
        egui::Window::new("Scene").show(&self.context(), |ui| {
            if ui.button("Load Scene").clicked() {
                SceneLoader::request_load();
            }
        });
    }
}
