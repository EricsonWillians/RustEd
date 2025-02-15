fn main() {
    // Instruct Cargo to re-run this build script if build.rs changes.
    println!("cargo:rerun-if-changed=build.rs");

    // If you have additional build steps (like embedding resources), add them here.
}