use std::{env, fs::File, io::Write, path::PathBuf};

fn main() {
    // Put the memory.x script somewhere the linker can find it
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();

    // Extend the linker search path
    println!("cargo:rustc-link-search={}", out.display());
}
