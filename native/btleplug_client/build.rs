// use std::env;
// use rustc_version::{version_meta, Channel};

// fn main() {
//     println!("cargo:rerun-if-changed=build.rs");

//     let target = env::var("TARGET").unwrap();
    
//     if target.contains("musl") {
//         println!("cargo:rustc-cfg=target_env=\"musl\"");
//     }
// }