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
//use std::error::Error;

use rustler::env::OwnedEnv;
use rustler::types::atom;
use rustler::types::LocalPid;
use rustler::{Atom, Encoder, Env, Error, Term};
use std::collections::HashMap;

// async fn get_central(manager: &Manager) -> Adapter {
//     let adapters = manager.adapters().await.unwrap();
//     adapters.into_iter().nth(0).unwrap()
// }

#[rustler::nif]
fn scan<'a>(env: Env<'a>) -> Result<Term<'a>, Atom> {
    let pid = env.pid();

    // task::spawn(async move {
    //     println!("Send async msg to async task");

    //     let mut msg_env = rustler::env::OwnedEnv::new();

    //     msg_env
    //         .send_and_clear(&pid, |env| (atoms::candidate_error()).encode(env))
    //         .unwrap();
    //     println!("After msg send");
    //     // match tx.send(Msg::AddIceCandidate(ice_candidate)).await {
    //     //     Ok(_) => (),
    //     //     Err(_err) => trace!("send error"),
    //     // }
    // });

    task::spawn(async move {
        println!("Test btleplug scan");

        let mut msg_env = rustler::env::OwnedEnv::new();

        let manager_result = Manager::new().await;

        match manager_result {
            Ok(manager) => {
                let central_result = get_central(&manager).await;

                match central_result {
                    Ok(central) => {
                        println!("Got central");

                        msg_env
                            .send_and_clear(&pid, |env| {
                                (atoms::btleplug_got_central(), "additional_data").encode(env)
                            })
                            .unwrap();

                        let mut events = central.events().await;

                        let _ = central.start_scan(ScanFilter::default()).await;

                        // Print based on whatever the event receiver outputs. Note that the event
                        // receiver blocks, so in a real program, this should be run in its own
                        // thread (not task, as this library does not yet use async channels).
                        while let Some(event) = events.as_mut().expect("REASON").next().await {
                            match event {
                                CentralEvent::DeviceDiscovered(id) => {
                                    let peripheral = central.peripheral(&id).await;
                                    // let properties = peripheral.properties().await;
                                    // let name = properties
                                    //     .and_then(|p| p.local_name)
                                    //     .map(|local_name| format!("Name: {local_name}"))
                                    //     .unwrap_or_default();
                                    // println!("DeviceDiscovered: {:?} {}", id, name);

                                    msg_env
                                        .send_and_clear(&pid, |env| {
                                            (
                                                atoms::btleplug_device_discovered(),
                                                print!("DeviceDiscovered: {:?}", id),
                                            )
                                                .encode(env)
                                        })
                                        .unwrap();
                                }
                                CentralEvent::StateUpdate(state) => {
                                    println!("AdapterStatusUpdate {:?}", state);
                                }
                                CentralEvent::DeviceConnected(id) => {
                                    println!("DeviceConnected: {:?}", id);
                                }
                                CentralEvent::DeviceDisconnected(id) => {
                                    println!("DeviceDisconnected: {:?}", id);
                                }
                                CentralEvent::ManufacturerDataAdvertisement {
                                    id,
                                    manufacturer_data,
                                } => {
                                    println!(
                                        "ManufacturerDataAdvertisement: {:?}, {:?}",
                                        id, manufacturer_data
                                    );
                                }
                                CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                                    println!(
                                        "ServiceDataAdvertisement: {:?}, {:?}",
                                        id, service_data
                                    );
                                }
                                CentralEvent::ServicesAdvertisement { id, services } => {
                                    let services: Vec<String> =
                                        services.into_iter().map(|s| s.to_short_string()).collect();
                                    println!("ServicesAdvertisement: {:?}, {:?}", id, services);
                                }
                                _ => {}
                            }
                        }
                    }

                    Err(e) => {
                        msg_env
                            .send_and_clear(&pid, |env| (atoms::btleplug_error()).encode(env))
                            .unwrap();
                        // Handle the error from get_central
                        println!("Failed to get central: {:?}", e);
                    }
                }
            }
            Err(e) => {
                // Handle the error from Manager::new()
                println!("Failed to create manager: {:?}", e);
            }
        }
    });

    Ok((atoms::ok(), pid).encode(env))
}

async fn get_central(manager: &Manager) -> Result<Adapter, Atom> {
    let adapters_result = manager.adapters().await;
    match adapters_result {
        Ok(adapters) => {
            if let Some(adapter) = adapters.into_iter().next() {
                Ok(adapter)
            } else {
                Err(atoms::no_adapters_found())
            }
        }
        Err(_e) => Err(atoms::btleplug_error()),
    }
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
