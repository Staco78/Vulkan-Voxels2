use std::{env, fs, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=shaders/");

    let shaders = fs::read_dir(env::var("CARGO_MANIFEST_DIR").unwrap() + "/shaders").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    for shader in shaders {
        let shader = shader.unwrap();
        let out_path = out_dir.clone() + "/" + shader.file_name().to_str().unwrap();
        let status = Command::new("glslc")
            .args([
                "-o",
                &out_path,
                shader.path().to_str().unwrap(),
                "--target-env=vulkan1.2",
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }
}
