// lib.rs

#![allow(unused_imports)]
#![allow(dead_code)]
//#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

mod atoms;
mod btle_manager;
mod btle_peripheral;
mod elixir_bridge;
mod task;
mod task_executor;

use crate::btle_manager::BtleManager;
use crate::elixir_bridge::ElixirBridge;
use crate::task_executor::spawn;
use futures::executor::block_on;
use rustler::{thread, Encoder, Env, Term, ResourceArc};
use std::collections::HashMap;

use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

use tokio::sync::Mutex as TokioMutex;
use tokio::runtime::{Runtime, Builder}; // Import Builder
use tokio::sync::MutexGuard;

use tokio::task::spawn_blocking; // Import spawn_blocking


//static BTLE_MANAGER: Mutex<Option<Arc<BtleManager>>> = Mutex::new(None);

#[rustler::nif]
fn init<'a>(env: Env<'a>) -> Term<'a> {
    println!("BtleManager init...");

    // let rt = Runtime::new().expect("Failed to create Tokio runtime for BTLE_MANAGER");
    // let manager = rt.block_on(async {
    //     BtleManager::new()
    //         .await
    //         .expect("Failed to create BtleManager")
    // });
    // println!("BtleManager initialized successfully.");
    // let mut btle_manager = BTLE_MANAGER.lock().unwrap();
    // *btle_manager = Some(Arc::new(manager));
    atoms::ok().encode(env)
}


#[rustler::nif]
fn new_ble_resource<'a>(env: Env<'a>) -> Term<'a> {
    let rt = Runtime::new().expect("Failed to create Tokio runtime for new");
    let manager = rt.block_on(async {
        BtleManager::new()
            .await
            .expect("Failed to create BtleManager")
    });
    let resource = ResourceArc::new(manager);
    //rustler::resource::register::<BtleManager>(env, atoms::btle_manager(), true);
    (atoms::ok(), resource).encode(env)
}


#[rustler::nif]
fn scan<'a>(env: Env<'a>, manager: ResourceArc<BtleManager>) -> Term<'a> {
    let rt = Runtime::new().expect("Failed to create Tokio runtime for scan");
    let device_ids: Result<Vec<String>, String> = rt.block_on(async {
        manager.scan().await
    });
    match device_ids {
        Ok(ids) => (atoms::ok(), ids).encode(env),
        Err(e) => (atoms::error(), e).encode(env),
    }
}

#[rustler::nif]
fn connect<'a>(env: Env<'a>, manager: ResourceArc<BtleManager>, device_id: String) -> Term<'a> {
    let rt = Runtime::new().expect("Failed to create Tokio runtime for connect");
    let result:Result<(),String> = rt.block_on(async {
        manager.connect_device(device_id).await
    });

    match result {
        Ok(_ok) => (atoms::ok(), atoms::btleplug_device_connected()).encode(env),
        Err(e) => {
             manager.bridge.send_message((
                atoms::btleplug_device_not_found(),
                format!("Device to connect not found: ID {:?}", device_id),
            ));
            (atoms::error(), e).encode(env)
        }
    }
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


fn on_load(env: Env, _info: Term) -> bool {
    rustler::resource!(BtleManager, env);
    true
}

rustler::init!("Elixir.RustlerBtleplug.Native", load = on_load);
