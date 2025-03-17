#![no_main]
// #![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
// #![allow(deprecated)]
#![allow(unused_must_use)]
#![allow(non_local_definitions)]
// #![allow(unexpected_cfgs)]
// #[cfg(not(clippy))]
// #[rustler::nif(schedule = "DirtyCpu")]

// MiMalloc won´t compile on Windows with the GCC compiler.
// On Linux with Musl it won´t load correctly.
#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "visionos",
    target_os = "watchos",
    all(target_os = "linux", not(target_env = "musl"))
)))]
use mimalloc::MiMalloc;

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "visionos",
    target_os = "watchos",
    all(target_os = "linux", not(target_env = "musl"))
)))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod atoms;
mod central_manager;
mod central_manager_finder;
mod central_manager_state;
mod central_manager_state_utils;
mod central_manager_utils;
mod logging;
mod peripheral;

extern crate rustler;
extern crate rustler_codegen;

use central_manager_state::CentralRef;
use log::{debug, info};
use once_cell::sync::Lazy;
use peripheral::*;
use rustler::{Env, Error as RustlerError, Term};
use std::collections::HashMap;
use tokio::runtime::Runtime;

// use std::os::raw::c_int;

pub static RUNTIME: Lazy<Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"));

fn on_load(env: Env, _info: Term) -> bool {
    // pretty_env_logger::init();
    logging::init_log();
    info!("Initializing Rust BLE NIF module ...");
    rustler::resource!(CentralRef, env);
    rustler::resource!(PeripheralRef, env);
    // rustler::resource!(GattPeripheralRef, env);
    debug!("Rust NIF BLE module loaded successfully.");
    true
}

#[rustler::nif]
pub fn test_string<'a>(env: Env<'a>, uuid: Term<'a>) -> Result<Term<'a>, RustlerError> {
    debug!(
        "Test string: uuid: {:?}, pid: {:?}",
        uuid,
        env.pid().as_c_arg()
    );
    Ok(uuid)
}

#[rustler::nif]
fn add(a: i64, b: i64) -> Result<i64, RustlerError> {
    Ok(a + b)
}

#[rustler::nif]
fn get_map() -> Result<HashMap<String, HashMap<String, String>>, RustlerError> {
    let mut map = HashMap::new();
    let mut inner_map = HashMap::new();
    inner_map.insert("inner_key1".to_string(), "inner_value1".to_string());
    inner_map.insert("inner_key2".to_string(), "inner_value2".to_string());
    map.insert("outer_key1".to_string(), inner_map);
    Ok(map)
}

// Static NIF entry points for OTP-27
/*#[no_mangle]
pub extern "C" fn nif_init() -> *const rustler::nif::ErlNifEntry {
    rustler::init!("Elixir.RustlerBtleplug.Native", load = on_load).as_ptr()
}*/

// #[no_mangle]
// pub extern "C" fn nif_version_2_15() -> c_int {
//     (2 << 16) as c_int | 15 as c_int
// }

// #[no_mangle]
// pub extern "C" fn libbtleplug_client_nif_init() -> i32 {
//     0 // Return 0 to indicate success
// }

// pub const NIF_MAJOR_VERSION: c_int = 2;
// pub const NIF_MINOR_VERSION: c_int = 15;
// find . -name nif_api.snippet.rs
rustler::init!("Elixir.RustlerBtleplug.Native", load = on_load);
