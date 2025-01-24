use std::{fs, path::Path};

use naga::{
    back::spv::{
        BindingMap, DebugInfo, SourceLanguage, WriterFlags, ZeroInitializeWorkgroupMemoryMode,
    },
    front::glsl::{Frontend, Options},
    proc::{BoundsCheckPolicies, BoundsCheckPolicy},
    valid::{Capabilities, ValidationFlags, Validator},
    ShaderStage,
};

struct Compiler {
    frontend: Frontend,
    validator: Validator,
}

fn compile_shader(path: impl AsRef<Path>, comp: &mut Compiler) -> Vec<u8> {
    let path = path.as_ref();
    let stage = match path.extension().map(|s| s.as_encoded_bytes()) {
        Some(b"frag") => ShaderStage::Fragment,
        Some(b"vert") => ShaderStage::Vertex,
        Some(b"comp") => ShaderStage::Compute,
        _ => panic!("{}: Can't determine stage from extension", path.display()),
    };
    let options = Options::from(stage);

    let glsl = fs::read_to_string(path).unwrap();
    let module = comp.frontend.parse(&options, &glsl).unwrap();
    let info = comp.validator.validate(&module).unwrap();
    let flags = if cfg!(debug_assertions) {
        println!("cargo::warning={}: Including debug labels", path.display());
        WriterFlags::DEBUG
    } else {
        WriterFlags::empty()
    };
    let check = if cfg!(debug_assertions) {
        BoundsCheckPolicy::ReadZeroSkipWrite
    } else {
        println!("cargo::warning={}: Bounds checks disabled", path.display());
        BoundsCheckPolicy::Unchecked
    };
    let options = naga::back::spv::Options {
        lang_version: (1, 0),
        flags,
        binding_map: BindingMap::new(),
        capabilities: None,
        bounds_check_policies: BoundsCheckPolicies {
            index: check,
            buffer: check,
            image_load: check,
            binding_array: check,
        },
        zero_initialize_workgroup_memory: ZeroInitializeWorkgroupMemoryMode::None,
        debug_info: cfg!(debug_assertions).then_some(DebugInfo {
            source_code: &glsl,
            file_name: path,
            language: SourceLanguage::GLSL,
        }),
    };
    let pipeline_options = naga::back::spv::PipelineOptions {
        shader_stage: stage,
        entry_point: "main".to_owned(),
    };

    let spv =
        naga::back::spv::write_vec(&module, &info, &options, Some(&pipeline_options)).unwrap();
    spv.iter()
        .fold(Vec::with_capacity(spv.len() * 4), |mut v, w| {
            v.extend_from_slice(&w.to_le_bytes());
            v
        })
}

fn main() {
    println!("cargo::rerun-if-changed=shaders");

    let flags = ValidationFlags::all();
    let capabilities = Capabilities::all();

    let mut compiler = Compiler {
        frontend: Frontend::default(),
        validator: Validator::new(flags, capabilities),
    };

    let shaders = fs::read_dir("shaders").unwrap();
    for entry in shaders.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(ft) if ft.is_file() => {
                let spv = compile_shader(&path, &mut compiler);
                let mut out = Path::new(&std::env::var_os("OUT_DIR").unwrap())
                    .join(path.file_name().unwrap());
                let mut extension = path.extension().unwrap().to_os_string();
                extension.push(".spv");
                assert!(out.set_extension(extension));
                fs::write(out, spv).unwrap();
            }
            _ => eprintln!("Skipping {}, not a regular file", path.display()),
        }
    }
}
