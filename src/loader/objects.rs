use gltf::Document;

pub struct GpuObject {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,

    pub transform: glam::Mat4,

    pub base_color: glam::Vec3A,
    pub emissive: glam::Vec3A,
    pub roughness: f32,
    pub metallic: f32,
}

pub fn load_objects(document: &Document, buffers: &[gltf::buffer::Data]) -> Vec<GpuObject> {
    let mut objects = Vec::new();

    for node in document.nodes() {
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

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

                let pbr = primitive.material().pbr_metallic_roughness();

                let base_color = glam::Vec3A::from_slice(&pbr.base_color_factor());
                let roughness = pbr.roughness_factor();
                let metallic = pbr.metallic_factor();
                let emissive = glam::Vec3A::from_array(primitive.material().emissive_factor());

                let object = GpuObject {
                    vertices,
                    indices,

                    transform,

                    base_color,
                    emissive,
                    roughness,
                    metallic,
                };

                objects.push(object);
            }
        }
    }

    for _mesh in document.meshes() {}

    objects
}
