#![allow(unused_imports)]
#![allow(dead_code)]
//#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

extern crate btleplug;
mod atoms;
mod task;
mod utils;

use btleplug::api::{
    bleuuid::BleUuid, Central, CentralEvent, CharPropFlags, Manager as _, Peripheral, ScanFilter,
    ValueNotification,
};
use btleplug::platform::{Adapter, Manager};

use futures::stream::StreamExt;

use btleplug::api::Characteristic;
use once_cell::sync::Lazy;
use rustler::env::OwnedEnv;
use rustler::types::LocalPid;
use rustler::{Atom, Encoder, Env, Term};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::{uuid, Uuid};

fn send_message<'a>(msg_env: &mut OwnedEnv, pid: &LocalPid, payload: (Atom, String)) {
    msg_env
        .send_and_clear(pid, |env| payload.encode(env))
        .unwrap();
}

// Struct to hold discovered Bluetooth objects
#[derive(Clone)]
pub struct BtleObject {
    pub peripheral_id: String, // Store the peripheral's ID
    pub services: Vec<btleplug::api::Service>,
    pub characteristics: Vec<Characteristic>,
}

impl BtleObject {
    pub fn new(
        peripheral_id: String,
        services: Vec<btleplug::api::Service>,
        characteristics: Vec<Characteristic>,
    ) -> Self {
        BtleObject {
            peripheral_id: peripheral_id,
            services: services,
            characteristics: characteristics,
        }
    }
}

// Thread-safe storage for discovered Bluetooth objects (No Rustler!)
pub struct BtleStorage {
    objects: Mutex<HashMap<String, BtleObject>>,
}

impl BtleStorage {
    pub fn new() -> Self {
        BtleStorage {
            objects: Mutex::new(HashMap::new()),
        }
    }

    pub fn add(&self, id: String, object: BtleObject) {
        let mut objects = self.objects.lock().unwrap();
        objects.insert(id, object);
    }

    pub fn get(&self, id: &str) -> Option<BtleObject> {
        let objects = self.objects.lock().unwrap();
        objects.get(id).cloned() // Clone the BtleObject if found
    }

    pub fn get_by_uuid(&self, uuid: &str) -> Option<BtleObject> {
        let objects = self.objects.lock().unwrap();
        for (_id, obj) in objects.iter() {
            for service in &obj.services {
                if service.uuid.to_string() == uuid {
                    return Some(obj.clone()); // Clone the BtleObject if a service UUID matches
                }
            }
            for characteristic in &obj.characteristics {
                if characteristic.uuid.to_string() == uuid {
                    return Some(obj.clone()); // Clone the BtleObject if a characteristic UUID matches
                }
            }
        }
        None
    }
}

// Static storage for BtleStorage
static BTLE_STORAGE: Lazy<Arc<BtleStorage>> = Lazy::new(|| Arc::new(BtleStorage::new()));

// Init function.
#[rustler::nif]
fn init<'a>(env: Env<'a>) -> Term<'a> {
    // The BTLE_STORAGE is initialized when first accessed, so calling it here is ok.
    let _ = &BTLE_STORAGE; // Force initialization of BTLE_STORAGE
    atoms::ok().encode(env)
}

#[rustler::nif]
fn scan<'a>(env: Env<'a>) -> Result<Term<'a>, Atom> {
    let pid = env.pid();

    let btle_storage_arc = BTLE_STORAGE.clone(); // Access the lazy static here

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
                                            let peripheral_id = id.to_string();
                                            let peripheral = Arc::new(peripheral); // Wrap in Arc

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
                                        Err(e) => {
                                            println!("PeripheralDiscovery Error {:?}", e);
                                            send_message(
                                                &mut msg_env,
                                                &pid,
                                                (
                                                    atoms::btleplug_device_discovery_error(),
                                                    format!(
                                                        "PeripheralDiscovered: Error {:?}, {:?}",
                                                        id, e
                                                    ),
                                                ),
                                            );
                                        }
                                    }
                                }
                                CentralEvent::StateUpdate(state) => {
                                    println!("AdapterStatusUpdate {:?}", state);

                                    send_message(
                                        &mut msg_env,
                                        &pid,
                                        (
                                            atoms::btleplug_adapter_status_update(),
                                            format!("AdapterStatusUpdate: State {:?}", state),
                                        ),
                                    );
                                }
                                CentralEvent::DeviceConnected(id) => {
                                    println!("DeviceConnected: {:?}", id);
                                    send_message(
                                        &mut msg_env,
                                        &pid,
                                        (
                                            atoms::btleplug_device_connected(),
                                            format!("DeviceConnected: ID {:?}", id),
                                        ),
                                    );
                                }
                                CentralEvent::DeviceDisconnected(id) => {
                                    println!("DeviceDisconnected: {:?}", id);
                                    send_message(
                                        &mut msg_env,
                                        &pid,
                                        (
                                            atoms::btleplug_device_disconnected(),
                                            format!("DeviceDisconnected: ID {:?}", id),
                                        ),
                                    );
                                }
                                CentralEvent::ManufacturerDataAdvertisement {
                                    id,
                                    manufacturer_data,
                                } => {
                                    println!(
                                        "ManufacturerDataAdvertisement: {:?}, {:?}",
                                        id, manufacturer_data
                                    );
                                    send_message(
                                        &mut msg_env,
                                        &pid,
                                        (
                                            atoms::btleplug_manufacturer_data_advertisement(),
                                            format!("ManufacturerDataAdvertisement: ID {:?}, DATA: {:?}", id, manufacturer_data),
                                        ),
                                    );
                                }
                                CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                                    println!(
                                        "ServiceDataAdvertisement: {:?}, {:?}",
                                        id, service_data
                                    );

                                    send_message(
                                        &mut msg_env,
                                        &pid,
                                        (
                                            atoms::btleplug_service_data_advertisement(),
                                            format!(
                                                "ServiceDataAdvertisement: ID {:?}, DATA: {:?}",
                                                id, service_data
                                            ),
                                        ),
                                    );
                                }
                                CentralEvent::ServicesAdvertisement { id, services } => {
                                    let services: Vec<String> =
                                        services.into_iter().map(|s| s.to_short_string()).collect();
                                    println!("ServicesAdvertisement: {:?}, {:?}", id, services);

                                    send_message(
                                        &mut msg_env,
                                        &pid,
                                        (
                                            atoms::btleplug_services_advertisement(),
                                            format!(
                                                "ServicesAdvertisement: ID {:?}, SERVICES: {:?}",
                                                id, services
                                            ),
                                        ),
                                    );
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

async fn get_peripheral_by_id(
    central: &Adapter,
    uuid: &Uuid,
) -> Option<btleplug::platform::Peripheral> {
    match central.peripherals().await {
        Ok(peripherals) => {

            println!("get_peripheral_by_id Ok peripherals: {}", peripherals.len());

            for peripheral in peripherals {
                println!(
                    "get_peripheral_by_id peripheral_id: {:?} uuid: {:?}",
                    peripheral.id().to_string(),
                    uuid.to_string()
                );

                if peripheral.id().to_string() == uuid.to_string() {
                    return Some(peripheral);
                }
            }
            None
        }
        Err(e) => {
            println!("get_peripheral_by_id Error: {:?}", e);
            None
        },
    }
}

#[rustler::nif]
fn connect<'a>(env: Env<'a>, peripheral_id: String) -> Result<Term<'a>, String> {
    println!("Connect to device {}", peripheral_id);

    let pid = env.pid();

    task::spawn(async move {
        let mut msg_env = rustler::env::OwnedEnv::new();

        match Manager::new().await {
            Ok(manager) => match get_central(&manager).await {
                Ok(central) => match Uuid::parse_str(&peripheral_id) {
                    Ok(peripheral_uuid) => {
                        if let Some(peripheral) =
                            get_peripheral_by_id(&central, &peripheral_uuid).await
                        {
                            let peripheral = Arc::new(peripheral);

                            match peripheral.connect().await {
                                Ok(_) => {
                                    println!("Connected to {:?}", peripheral_id);
                                    send_message(
                                        &mut msg_env,
                                        &pid,
                                        (
                                            atoms::btleplug_device_connected(),
                                            format!("Connected to {}", peripheral_id),
                                        ),
                                    );
                                }
                                Err(e) => {
                                    println!("Failed to connect: {:?}", e);
                                    send_message(
                                        &mut msg_env,
                                        &pid,
                                        (
                                            atoms::btleplug_error(),
                                            format!("Failed to connect: {:?}", e),
                                        ),
                                    );
                                }
                            }
                        } else {
                            println!("Peripheral not found for ID {}", peripheral_id);
                            send_message(
                                &mut msg_env,
                                &pid,
                                (
                                    atoms::btleplug_error(),
                                    format!("Peripheral not found for ID {}", peripheral_id),
                                ),
                            );
                        }
                    }
                    Err(e) => {
                        println!(
                            "Peripheral error parsing uuid {:?}, e: {:?}",
                            peripheral_id, e
                        );
                    }
                },
                Err(e) => {
                    println!("Failed to get central: {:?}", e);
                    send_message(
                        &mut msg_env,
                        &pid,
                        (atoms::btleplug_error(), "Central error".to_string()),
                    );
                }
            },
            Err(e) => {
                println!("Failed to create manager: {:?}", e);
                send_message(
                    &mut msg_env,
                    &pid,
                    (
                        atoms::btleplug_error(),
                        "Manager creation error".to_string(),
                    ),
                );
            }
        }
    });

    Ok((atoms::ok(), "Connection started").encode(env))
}

async fn get_central(manager: &Manager) -> Result<Adapter, Atom> {
    let adapters_result = manager.adapters().await;
    match adapters_result {
        Ok(adapters) => {
            if let Some(adapter) = adapters.into_iter().next() {
                Ok(adapter)
            } else {
                Err(atoms::btleplug_no_adapters_found())
            }
        }
        Err(_e) => Err(atoms::btleplug_error()),
    }
}

fn get_btle_object_by_uuid<'a>(env: Env<'a>, uuid: Term<'a>) -> Result<Term<'a>, Atom> {
    let uuid_str: String = rustler::Decoder::decode(uuid).unwrap();

    let btle_storage = &BTLE_STORAGE;

    match btle_storage.get_by_uuid(&uuid_str) {
        Some(btle_object) => {
            // Return a success atom along with the peripheral ID.
            Ok((atoms::ok(), btle_object.peripheral_id).encode(env))
        }
        None => Err(atoms::not_found()),
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
}

// [add, get_map, init, scan]
rustler::init!("Elixir.RustlerBtleplug.Native");
// , [add, get_map, init, scan]
