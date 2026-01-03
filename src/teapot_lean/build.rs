use shader_slang::{self as slang, Downcast};
use std::fs;
use std::path::Path;

fn visit_dirs(
    dir: &Path,
    cb: &dyn Fn(&std::path::PathBuf, shaderc::ShaderKind),
) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                let path_buf = entry.path();
                if let Some(shader_kind) = get_shader_kind(&path_buf) {
                    cb(&path_buf, shader_kind);
                }
            }
        }
    }
    Ok(())
}

fn get_shader_kind(path_buf: &std::path::PathBuf) -> Option<shaderc::ShaderKind> {
    let extension = path_buf
        .extension()
        .expect("file has no extension")
        .to_str()
        .expect("extension cannot be converted to &str");

    match extension {
        "vert" => Some(shaderc::ShaderKind::Vertex),
        "frag" => Some(shaderc::ShaderKind::Fragment),
        "tese" => Some(shaderc::ShaderKind::TessEvaluation),
        "tesc" => Some(shaderc::ShaderKind::TessControl),
        _ => None,
    }
}

fn compile_shader(path_buf: &std::path::PathBuf, shader_kind: shaderc::ShaderKind) {
    let shader_str = fs::read_to_string(path_buf)
        .expect(&format!("failed to read shader {:?} to string", path_buf));

    let compiler = shaderc::Compiler::new().expect("failed to create shader compilier");

    println!("compiling shader {:?}", path_buf);

    let spv = compiler
        .compile_into_spirv(
            &shader_str,
            shader_kind,
            &path_buf.to_str().unwrap(),
            "main",
            None,
        )
        .expect(&format!("failed to compile shader {:?}", path_buf));

    let mut file_name = path_buf
        .file_name()
        .expect("shader file should have a name")
        .to_os_string();

    println!("cargo:rerun-if-changed={}", path_buf.display());

    file_name.push(".spv");

    let mut spv_path = path_buf
        .parent()
        .expect("failed to get shader file parent folder")
        .join("..")
        .join("..")
        .join("..")
        .join("..")
        .join("shaders")
        .join("glsl");

    std::fs::create_dir_all(spv_path.clone()).expect(&format!(
        "failed to create directory for shader {:?}",
        path_buf
    ));

    spv_path.push(file_name);

    fs::write(spv_path, spv.as_binary_u8()).expect("failed to write shader binary");
}

fn visit_dirs_slang(dir: &Path, cb: &dyn Fn(&str, &slang::GlobalSession)) -> std::io::Result<()> {
    let global_session = slang::GlobalSession::new().unwrap();

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                visit_dirs_slang(&path, cb)?;
            } else {
                let path_buf = entry.path();
                if get_shader_kind_is_slang(&path_buf) {
                    let tmp_dir = path_buf.to_str();
                    match tmp_dir {
                        None => panic!("new path is not a valid UTF-8 sequence"),
                        Some(s) => cb(s, &global_session),
                    }
                }
            }
        }
    }
    Ok(())
}

fn get_shader_kind_is_slang(path_buf: &std::path::PathBuf) -> bool {
    let extension = path_buf
        .extension()
        .expect("file has no extension")
        .to_str()
        .expect("extension cannot be converted to &str");

    let is_slang = match extension {
        "slang" => true,
        _ => false,
    };

    is_slang
}

fn compile_slang(dir: &str, global_session: &slang::GlobalSession) {
    let search_path = std::ffi::CString::new(dir).unwrap();

    let session_options = slang::CompilerOptions::default()
        .optimization(slang::OptimizationLevel::High)
        // .optimization(slang::OptimizationLevel::None)
        .matrix_layout_row(true);

    let target_desc = slang::TargetDesc::default()
        .format(slang::CompileTarget::Spirv)
        .profile(global_session.find_profile("glsl_450"));

    let targets = [target_desc];
    let search_paths = [search_path.as_ptr()];

    let session_desc = slang::SessionDesc::default()
        .targets(&targets)
        .search_paths(&search_paths)
        .options(&session_options);

    let session = global_session.create_session(&session_desc).unwrap();
    let module = session.load_module(dir).unwrap();
    let entry_point = module.find_entry_point_by_name("main").unwrap();

    println!("compiling shader {:?}", dir);
    let program = session
        .create_composite_component_type(&[
            module.downcast().clone(),
            entry_point.downcast().clone(),
        ])
        .unwrap();

    let linked_program = program.link().unwrap();

    let reflection = linked_program.layout(0).unwrap();

    let shader_bytecode = linked_program.entry_point_code(0, 0).unwrap();

    let path_buf = Path::new(dir);

    let mut file_name = path_buf
        .file_stem()
        .expect("shader file should have a name")
        .to_os_string();

    println!("cargo:rerun-if-changed={}", path_buf.display());

    file_name.push(".spv");

    let mut spv_path = path_buf
        .parent()
        .expect("failed to get shader file parent folder")
        .join("..")
        .join("..")
        .join("..")
        .join("..")
        .join("shaders")
        .join("slang");

    std::fs::create_dir_all(spv_path.clone()).expect(&format!(
        "failed to create directory for shader {:?}",
        path_buf
    ));

    spv_path.push(file_name);

    // println!("{}", spv_path.display());
    fs::write(spv_path, shader_bytecode.as_slice().to_vec())
        .expect("failed to write shader binary");
}

fn main() -> Result<(), i32> {
    let shaders_dir = Path::new("shaders/glsl");

    if let Err(_) = visit_dirs(shaders_dir, &compile_shader) {
        return Err(1);
    }

    let slang_dir = Path::new("shaders/slang");
    if let Err(_) = visit_dirs_slang(slang_dir, &compile_slang) {
        return Err(1);
    }

    Ok(())
}
