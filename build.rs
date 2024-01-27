use shaderc::ShaderKind;
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

type ShaderInfo = (&'static str, ShaderKind);

const SHADER_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders");
const SHADERS: [ShaderInfo; 2] = [
    ("interface.frag", ShaderKind::Fragment),
    ("interface.vert", ShaderKind::Vertex),
];

fn main() {
    println!("cargo:rerun-if-changed=shaders/");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let compiler = shaderc::Compiler::new().unwrap();

    for (path, stage) in SHADERS {
        let (code, path) = load_shader(path);
        let name = path.file_name().unwrap().to_str().unwrap();

        let mut compile_options = shaderc::CompileOptions::new().unwrap();
        compile_options.set_generate_debug_info();
        compile_options.set_optimization_level(shaderc::OptimizationLevel::Performance);

        let artifact = compiler
            .compile_into_spirv(&code, stage, name, "main", Some(&compile_options))
            .unwrap();
        let spv = artifact.as_binary();

        fs::create_dir_all(&out_dir).unwrap();
        let module_dir = Path::new(&out_dir).join(format!("{}.rs", name));
        let mut module = File::create(module_dir).unwrap();

        write_spirv(&mut module, &spv);
    }
}

fn load_shader(name: &str) -> (String, PathBuf) {
    let src_path = std::path::Path::new(SHADER_DIR).join(name);
    let mut code = String::new();
    std::io::Read::read_to_string(&mut File::open(src_path.clone()).unwrap(), &mut code).unwrap();

    (code, src_path)
}

fn write_spirv(module: &mut File, code: &[u32]) {
    writeln!(
        module,
        "pub const CODE: [u32; {}] = {:?};",
        code.len(),
        code
    )
    .unwrap();
}
