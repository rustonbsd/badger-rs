use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let default_lib_dir = manifest_dir.join("./lib");
    let lib_dir = env::var("BADGER_FFI_LIB_DIR")
        .map(PathBuf::from)
        .unwrap_or(default_lib_dir);

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=badgerffi");
    println!("cargo:rerun-if-env-changed=BADGER_FFI_LIB_DIR");

    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
}
