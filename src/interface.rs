use crate::{loader::SceneLoader, world::World};
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

    pub fn handle_event(&mut self, window: &Window, event: WindowEvent) {
        let _response = self.window_integration.on_window_event(window, &event);
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
            ui.label("Position");
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut world.camera.position.x));

                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut world.camera.position.y));

                ui.label("Z:");
                ui.add(egui::DragValue::new(&mut world.camera.position.z));
            });

            ui.label("Rotation");
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut world.camera.rotation.x).speed(0.1));

                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut world.camera.rotation.y).speed(0.1));

                ui.label("Z:");
                ui.add(egui::DragValue::new(&mut world.camera.rotation.z).speed(0.1));
            });

            ui.horizontal(|ui| {
                ui.label("Y FOV: ");
                ui.add(egui::DragValue::new(&mut world.camera.fov));
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
