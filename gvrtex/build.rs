fn main() {
    let _ = cxx_build::bridge("src/shared.rs");

    println!("cargo:rerun-if-changed=src/shared.rs");
}
