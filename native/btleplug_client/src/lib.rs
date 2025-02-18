extern crate btleplug;
//extern crate tokio;
//use crate::atoms;
//use crate::state::Ref;
// mod atoms;
// mod task;

//pub mod atoms;
//pub mod task;
mod atoms;
mod task;
//use crate::atoms;
//use crate::{atoms, task};

use btleplug::api::{
    bleuuid::BleUuid, Central, CentralEvent, Manager as _, Peripheral, ScanFilter,
};
use btleplug::platform::{Adapter, Manager};
use futures::stream::StreamExt;
use std::error::Error;

use rustler::env::OwnedEnv;
use rustler::types::atom;
use rustler::types::LocalPid;
//use rustler::types::Pid;
use rustler::{Encoder, Env, Term};
use std::collections::HashMap;


async fn get_central(manager: &Manager) -> Adapter {
    let adapters = manager.adapters().await.unwrap();
    adapters.into_iter().nth(0).unwrap()
}


#[rustler::nif]
//fn scan<'a>(env: Env<'a>) -> Term<'a> {
fn scan<'a>(env: Env<'a>) -> Result<rustler::Term<'a>, LocalPid> {
    let mut msg_env = rustler::env::OwnedEnv::new();
    let pid = env.pid();

    task::spawn(async move {
        println!("Send async msg to async task");
        msg_env.send_and_clear(&pid, |env| {
        (atoms::candidate_error()).encode(env)
    }).unwrap();
        println!("After msg send");
        // match tx.send(Msg::AddIceCandidate(ice_candidate)).await {
        //     Ok(_) => (),
        //     Err(_err) => trace!("send error"),
        // }
    });

    // Return a proper term instead of Ok(_)
    //(atoms::ok()).encode(env)

    Ok((atoms::ok(), pid).encode(env))

    //"test".to_string()
    //let pid = env.get_pid();

    // if pid {
    //     println!("Hello, world!");
    // }

    //fn scan() -> String {
    // let manager = Manager::new();
    // let adapter_list = manager.adapters();
    // if adapter_list.is_empty() {
    //     eprintln!("No Bluetooth adapters found");
    // }

    // for adapter in adapter_list.iter() {
    //     println!("Starting scan on {}...", adapter.adapter_info());
    //     adapter
    //         .start_scan(ScanFilter::default())
    //         .expect("Can't scan BLE adapter for connected devices...");

    //     let peripherals = adapter.peripherals();
    //     if peripherals.is_empty() {
    //         eprintln!("->>> BLE peripheral devices were not found, sorry. Exiting...");
    //     } else {
    //         // All peripheral devices in range
    //         for peripheral in peripherals.iter() {
    //             let properties = peripheral.properties();
    //             let is_connected = peripheral.is_connected();
    //             let local_name = properties
    //                 .unwrap()
    //                 .local_name
    //                 .unwrap_or(String::from("(peripheral name unknown)"));
    //             println!(
    //                 "Peripheral {:?} is connected: {:?}",
    //                 local_name, is_connected
    //             );
    //             if !is_connected {
    //                 println!("Connecting to peripheral {:?}...", &local_name);
    //                 if let Err(err) = peripheral.connect() {
    //                     eprintln!("Error connecting to peripheral, skipping: {}", err);
    //                     continue;
    //                 }
    //             }
    //             let is_connected = peripheral.is_connected();
    //             println!(
    //                 "Now connected ({:?}) to peripheral {:?}...",
    //                 is_connected, &local_name
    //             );
    //             peripheral.discover_services();
    //             println!("Discover peripheral {:?} services...", &local_name);
    //             for service in peripheral.services() {
    //                 println!(
    //                     "Service UUID {}, primary: {}",
    //                     service.uuid, service.primary
    //                 );
    //                 for characteristic in service.characteristics {
    //                     println!("  {:?}", characteristic);
    //                 }
    //             }
    //             if is_connected {
    //                 println!("Disconnecting from peripheral {:?}...", &local_name);
    //                 peripheral
    //                     .disconnect()
    //                     .expect("Error disconnecting from BLE peripheral");
    //             }
    //         }
    //     }
    // }
    // "test".to_string()
    //Ok(())
}

pub struct State<'a> {
    pub opts: Term<'a>,
    pub pid: LocalPid,
    // apis: HashMap<String, Arc<API>>,
    // media_engines: HashMap<String, MediaEngine>,
    // peer_connections: HashMap<String, Sender<peer_connection::Msg>>,
    // registries: HashMap<String, Registry>,
    // local_static_sample_tracks: HashMap<String, Arc<TrackLocalStaticSample>>,
}

impl<'a> State<'a> {
    fn new(opts: Term<'a>, pid: LocalPid) -> Self {
        State {
            opts,
            pid,
            // apis: HashMap::new(),
            // media_engines: HashMap::new(),
            // peer_connections: HashMap::new(),
            // registries: HashMap::new(),
            // local_static_sample_tracks: HashMap::new(),
        }
    }

    //***** API

    // pub(crate) fn add_api(&mut self, uuid: &str, api: API) -> &mut State {
    //     self.apis.insert(uuid.to_owned(), Arc::new(api));
    //     self
    // }
}

#[rustler::nif(name = "__init__")]
fn init<'a>(env: Env<'a>, opts: Term<'a>) -> Term<'a> {
    // let config = match Config::parse(env, opts) {
    //     Err(error) => return (atoms::error(), error).encode(env),
    //     Ok(config) => config,
    // };

    let state = State::new(opts, env.pid());
    //let resource = ResourceArc::new(Ref(Arc::new(Mutex::new(state))));

    (atoms::ok()).encode(env)
}

// #[rustler::nif]
// fn init() {
//     // Initialization code here
// }

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
}

// [add, get_map, init, scan]
rustler::init!("Elixir.RustlerBtleplug.Native");
