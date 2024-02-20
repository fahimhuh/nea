use crate::{
    input::{Input, Inputs},
    loader::SceneLoader,
    world::World,
};
use winit::{event::WindowEvent, window::Window};


// This struct is used to store the interface, which is the construct that integrates this program with the egui library
// It contains all the egui objects and state that are used to render the interface, so the usage of the library is contained
// into one module.
pub struct Interface {
    // The egui context is the main object that is used to interface with the egui library
    interface_context: egui::Context,
    // The window integration is the object that is used to interface between the egui library and the winit library
    window_integration: egui_winit::State,

    // The last output is the last frame of the interface that was rendered
    last_output: egui::FullOutput,
}

impl Interface {
    // Initialise the interface with the given window
    pub fn new(window: &Window) -> Self {
        // Create the egui context and window integration objects
        let interface_context = egui::Context::default();
        let window_integration = egui_winit::State::new(
            interface_context.clone(),
            interface_context.viewport_id(),
            &window,
            None,
            None,
        );

        // Return the interface object
        Self {
            interface_context,
            window_integration,
            last_output: egui::FullOutput::default(),
        }
    }

    // This function is used to handle window events that are recieved from the window
    // and update the interface accordingly.
    // Anything that is not consumed by the interface is sent to the input queue
    pub fn handle_event(&mut self, window: &Window, event: WindowEvent, inputs: &Inputs) {
        // Handle the window event with the window integration object
        let response = self.window_integration.on_window_event(window, &event);

        // If the event was not consumed by the interface, send it to the input queue
        if !response.consumed {
            inputs.broadcaster.try_send(Input::from_window_event(event));
        }
    }

    // This function is used to update the interface
    // It is called once per frame, and is used to update the interface with the latest input events
    // and the latest state of the world
    pub fn update(&mut self, window: &Window, world: &mut World) {
        // Get the raw input from the window integration object
        let raw_input = self.window_integration.take_egui_input(window);
        // Begin the frame with the raw input
        self.interface_context.begin_frame(raw_input);

        // Draw the user interface
        self.scene_ui();
        self.camera_ui(world);

        // End the frame and get the output
        let output = self.interface_context.end_frame();
        self.window_integration
            .handle_platform_output(window, output.platform_output.clone());

        self.last_output = output;
    }

    // Get an immutable reference to the internal egui context, so we dont accidentally modify it
    pub fn context(&self) -> &egui::Context {
        &self.interface_context
    }

    // Helper function to get the last output of the interface
    pub fn take_last_output(&mut self) -> egui::FullOutput {
        std::mem::take(&mut self.last_output)
    }

    // Draw the camera UI
    pub fn camera_ui(&mut self, world: &mut World) {
        egui::Window::new("Camera").show(&self.context(), |ui| {
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

    // Draw the scene description UI
    pub fn scene_ui(&mut self) {
        egui::Window::new("Scene").show(&self.context(), |ui| {
            if ui.button("Load Scene").clicked() {
                // Spawn a new thread to load the scene
                SceneLoader::request_load();
            }
        });
    }
}
