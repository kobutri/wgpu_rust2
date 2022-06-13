extern crate core;

use std::{env, fs};
use std::path::Path;

const VERTEX_CODE: &str = "\
#version 450
\
\
void main() {}\
";

const FRAGMENT_CODE: &str = "\
#version 450
\
\
void main() {}\
";

fn compile(code: &str, name: &str, kind: shaderc::ShaderKind) -> shaderc::CompilationArtifact {
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_optimization_level(shaderc::OptimizationLevel::Performance);
    let mut compiler = shaderc::Compiler::new().unwrap();
    let res = compiler.compile_into_spirv(code, kind, name, "main", Some(&options)).unwrap();
    if res.get_num_warnings() != 0 {
        eprintln!("{}", res.get_warning_messages());
    }
    res
}


fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let vertex_path = Path::new(&out_dir).join("fallback_vert.spv");
    let fragment_path = Path::new(&out_dir).join("fallback_frag.spv");
    let vertex_shader = compile(VERTEX_CODE, "fallback_shader.vert", shaderc::ShaderKind::Vertex);
    let fragment_shader = compile(FRAGMENT_CODE, "fallback_shader.frag", shaderc::ShaderKind::Fragment);
    fs::write(&vertex_path, vertex_shader.as_binary_u8()).unwrap();
    fs::write(&fragment_path, fragment_shader.as_binary_u8()).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}