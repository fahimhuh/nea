use gltf::Document;

pub struct GpuObject {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,

    pub transform: glam::Mat4,
}

pub fn load_objects(document: &Document, buffers: &[gltf::buffer::Data]) -> Vec<GpuObject> {
    let mut objects = Vec::new();

    for node in document.nodes() {
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                // FIXME: This may need to be transposed
                let transform = glam::Mat4::from_cols_array_2d(&node.transform().matrix());

                let vertices = reader
                    .read_positions()
                    .unwrap()
                    .into_iter()
                    .flatten()
                    .collect::<Vec<f32>>();

                let indices = reader
                    .read_indices()
                    .unwrap()
                    .into_u32()
                    .collect::<Vec<u32>>();

                let object = GpuObject {
                    vertices,
                    indices,
                    transform,
                };

                objects.push(object);
            }
        }
    }

    for _mesh in document.meshes() {}

    objects
}
