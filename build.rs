use shaderc::ShaderKind;
use std::{
    env,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

type ShaderInfo = (&'static str, ShaderKind);

const SHADER_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders");
const SHADERS: [ShaderInfo; 3] = [
    ("interface/interface.frag", ShaderKind::Fragment),
    ("interface/interface.vert", ShaderKind::Vertex),
    ("raytracer.comp", ShaderKind::Compute),
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
        compile_options.set_include_callback(shader_include_callback);

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

pub fn shader_include_callback(
    name: &str,
    _kind: shaderc::IncludeType,
    _original: &str,
    _depth: usize,
) -> shaderc::IncludeCallbackResult {
    // Search for shaders relative to the project directory
    const SHADER_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders");

    let mut path = PathBuf::from(SHADER_DIR);
    path.push(name);

    let mut content = String::new();
    let mut file = File::open(path).unwrap();
    file.read_to_string(&mut content).unwrap();

    let resolved_name = name.to_string();

    Ok(shaderc::ResolvedInclude {
        resolved_name,
        content,
    })
}
