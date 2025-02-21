#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(deprecated)]
#![allow(unused_must_use)]
#![allow(non_local_definitions)]
// #[rustler::nif(schedule = "DirtyCpu")]

mod atoms;
mod central_manager;
mod peripheral;

#[macro_use]
extern crate rustler;
#[macro_use]
extern crate rustler_codegen;

use central_manager::*;
use once_cell::sync::Lazy;
use peripheral::*;
use rustler::{Env, Error as RustlerError, Term};
use std::collections::HashMap;
use tokio::runtime::Runtime;

pub static RUNTIME: Lazy<Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"));

fn on_load(env: Env, _info: Term) -> bool {
    println!("[Rust] Initializing Rust NIF module...");
    rustler::resource!(CentralRef, env);
    rustler::resource!(PeripheralRef, env);
    println!("[Rust] Rust NIF module loaded successfully.");
    true
}

#[rustler::nif]
pub fn test_string<'a>(env: Env<'a>, uuid: Term<'a>) -> Result<Term<'a>, RustlerError> {
    println!("[Rust] Test string: {:?}", uuid);
    Ok(uuid)
}

#[rustler::nif]
fn add(a: i64, b: i64) -> i64 {
    a * b
}

#[rustler::nif]
fn get_map() -> HashMap<String, HashMap<String, String>> {
    let mut map = HashMap::new();
    let mut inner_map = HashMap::new();
    inner_map.insert("inner_key1".to_string(), "inner_value1".to_string());
    inner_map.insert("inner_key2".to_string(), "inner_value2".to_string());
    map.insert("outer_key1".to_string(), inner_map);
    map
    //atoms::ok().encode(env)
}

rustler::init!("Elixir.RustlerBtleplug.Native", load = on_load);
