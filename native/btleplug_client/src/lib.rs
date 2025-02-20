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
use elixir_bridge::*;
use peripheral::*;
use rustler::{Env, Error as RustlerError, Term};

fn on_load(env: Env, _info: Term) -> bool {
    println!("[Rust] Initializing Rust NIF module...");
    println!("[Rust] Rust NIF module loaded successfully.");
    true
}

#[rustler::nif]
pub fn test_string<'a>(env: Env<'a>, uuid: Term<'a>) -> Result<Term<'a>, RustlerError> {
    println!("[Rust] Test string: {:?}", uuid);
    Ok(uuid)
}

rustler::init!("Elixir.RustlerBtleplug.Native", load = on_load);
