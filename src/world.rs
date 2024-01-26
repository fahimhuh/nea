#[derive(Default, Debug)]
pub struct Object {}

#[derive(Default, Debug)]
pub struct Camera {}

pub struct World {
    camera: Camera,
    objects: Vec<Object>,
}

impl World {
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            objects: Vec::default(),
        }
    }
}
