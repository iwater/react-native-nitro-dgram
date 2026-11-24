use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let package_name = env::var("CARGO_PKG_NAME").unwrap();
    let output_file = PathBuf::from(&crate_dir)
        .join("include")
        .join(format!("{}.hpp", "UdpBindings"));

    // Ensure the include directory exists
    std::fs::create_dir_all(output_file.parent().unwrap()).unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::Cxx)
        .with_include_guard("UDP_BINDINGS_H")
        .with_namespace("udp")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(output_file);
}
