use std::env;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".to_string());
    let asm_src = "src/wavesort.asm";
    let obj_file = format!("{}/wavesort.o", out_dir);
    let lib_file = "libwavesort.a";

    // 1. Assemble the ASM file using NASM
    // Detect OS to set the correct format
    let format = if cfg!(target_os = "macos") {
        "macho64"
    } else {
        "elf64"
    };

    let nasm_opt = if profile == "debug" { "-O0" } else { "-O3" };
    let status = Command::new("nasm")
        .args(&["-f", format, nasm_opt, asm_src, "-o", &obj_file])
        .status()
        .expect("Failed to run nasm. Is it installed?");

    if !status.success() {
        panic!("NASM compilation failed");
    }

    // 2. Create a static library (archive) from the object file
    let status = Command::new("ar")
        .args(&["crus", &format!("{}/{}", out_dir, lib_file), &obj_file])
        .status()
        .expect("Failed to run ar");

    if !status.success() {
        panic!("Failed to create static library");
    }

    // 3. Tell Cargo to link the library
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=wavesort");

    // Re-run build script if the ASM file changes
    println!("cargo:rerun-if-changed={}", asm_src);
    println!("cargo:rerun-if-env-changed=PROFILE");
}
