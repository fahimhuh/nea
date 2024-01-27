use crate::loader::SceneLoader;

pub struct Raytracer {}

impl Raytracer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(&mut self) {
        if let Some(scene) = SceneLoader::poll() {
            log::info!("TADA");
        }
    }
}
