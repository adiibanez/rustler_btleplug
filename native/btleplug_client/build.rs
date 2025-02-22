use rustc_version::{version_meta, Channel};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if let Ok(target) = std::env::var("CARGO_CFG_TARGET_ENV") {
        if target == "musl" {
            println!("cargo:rustc-cfg=musl_target");
            println!("cargo:rustc-cdylib-link-arg=-static");
        }
    }
}