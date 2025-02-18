extern crate btleplug;
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
use rustler::types::LocalPid;
use rustler::{Atom, Encoder, Env, Term};
use std::collections::HashMap;

fn send_message<'a>(msg_env: &mut OwnedEnv, pid: &LocalPid, payload: (Atom, String)) {
    msg_env
        .send_and_clear(pid, |env| payload.encode(env))
        .unwrap();
}

#[rustler::nif]
fn scan<'a>(env: Env<'a>) -> Result<Term<'a>, Atom> {
    let pid = env.pid();

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

                        send_message(
                            &mut msg_env,
                            &pid,
                            (atoms::btleplug_got_central(), "additional_data".to_string()),
                        );

                        let mut events = central.events().await;

                        let _ = central.start_scan(ScanFilter::default()).await;

                        while let Some(event) = events.as_mut().expect("REASON").next().await {
                            match event {
                                CentralEvent::DeviceDiscovered(id) => {
                                    let peripheral_result = central.peripheral(&id).await;

                                    match peripheral_result {
                                        Ok(peripheral) => {
                                            let peripheral_is_connected: bool =
                                                peripheral.is_connected().await.expect("REASON");

                                            let properties =
                                                peripheral.properties().await.expect("REASON");

                                            let name = properties
                                                .and_then(|p| p.local_name)
                                                .map(|local_name| format!("Name: {local_name}"))
                                                .unwrap_or_default();

                                            send_message(
                                                &mut msg_env,
                                                &pid,
                                                (
                                                    atoms::btleplug_device_discovered(),
                                                    format!(
                                                        "PeripheralDiscovered: {:?}, {:?} {:?}",
                                                        id, name, peripheral_is_connected
                                                    ),
                                                ),
                                            );
                                        }
                                        Err(e) => {}
                                    }
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
                        send_message(
                            &mut msg_env,
                            &pid,
                            (atoms::btleplug_error(), "".to_string()),
                        );
                        println!("Failed to get central: {:?}", e);
                    }
                }
            }
            Err(e) => {
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
