#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(deprecated)]
#![allow(unused_must_use)]
#![allow(non_local_definitions)]

use futures::{channel::mpsc::channel, prelude::*};
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, ResourceArc, Term};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, atomic},
};
use uuid::{Uuid, Builder, Bytes, Variant};

use bluster::{
    gatt::{
        characteristic,
        characteristic::Characteristic,
        descriptor,
        descriptor::Descriptor,
        event::{Event, Response},
        service::Service,
    },
    Peripheral, SdpShortUuid,
};

use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tokio::time::Duration;

use crate::RUNTIME;

pub struct GattPeripheralRef(pub(crate) Arc<Mutex<GattPeripheralState>>);

pub struct GattPeripheralState {
    pub pid: LocalPid,
    pub peripheral_name: String,
    pub peripheral: Arc<RwLock<Peripheral>>,
}

impl GattPeripheralState {
    pub fn new(pid: LocalPid, peripheral_name: String, peripheral: Arc<RwLock<Peripheral>>) -> Self {
        GattPeripheralState {
            pid,
            peripheral_name,
            peripheral,
        }
    }
}

impl Drop for GattPeripheralState {
    fn drop(&mut self) {
        println!("[Rust] GattPeripheralState destructor called.");
    }
}

#[rustler::nif]
pub fn create_gatt_peripheral(
    env: Env,
    peripheral_name: String,
    advertising_duration_ms: u64,
) -> Result<ResourceArc<GattPeripheralRef>, rustler::Error> {
    println!("[Rust] Creating GATT peripheral with name: {}", peripheral_name);
    
    let peripheral = RUNTIME.block_on(Peripheral::new())
        .map_err(|_| RustlerError::Atom("failed_to_create_peripheral"))?;

    println!("[Rust] Peripheral created successfully");

    let peripheral_arc = Arc::new(RwLock::new(peripheral));  
    let state = GattPeripheralState::new(env.pid(), peripheral_name.clone(), peripheral_arc);
    let resource = ResourceArc::new(GattPeripheralRef(Arc::new(Mutex::new(state))));

    let (sender_characteristic, receiver_characteristic) = channel(1);
    let (sender_descriptor, receiver_descriptor) = channel(1);

    println!("[Rust] Creating characteristics and descriptors...");
    let characteristics = create_characteristics(sender_characteristic, sender_descriptor);
    let service_uuid = short_uuid_to_uuid(0x1234);
    let state_ref = resource.0.clone();

    let (peripheral_name, peripheral) = {
        let state_guard = state_ref.lock().unwrap();
        println!("[Rust] Locked state. Peripheral name: {}", state_guard.peripheral_name);
        (state_guard.peripheral_name.clone(), state_guard.peripheral.clone())
    };

    RUNTIME.spawn(async move {
        println!("[Rust] Spawning async task for service setup and advertising...");
        if let Err(e) = setup_service_and_start_advertising(peripheral, service_uuid, characteristics, peripheral_name).await {
            println!("[Rust] Failed to setup service and start advertising: {:?}", e);
            return;
        }

        println!("[Rust] Services setup completed. Advertising started...");

        futures::join!(
            handle_characteristics(receiver_characteristic),
            handle_descriptors(receiver_descriptor),
            tokio::time::sleep(Duration::from_millis(advertising_duration_ms))
        );

        println!("[Rust] GATT peripheral advertising timeout reached");

        if let Err(e) = stop_advertising(state_ref.clone()).await {
            println!("[Rust] Failed to stop advertising: {:?}", e);
        }
    });

    println!("[Rust] GATT peripheral created successfully");
    Ok(resource)
}

async fn setup_service_and_start_advertising(
    peripheral: Arc<RwLock<Peripheral>>,  
    service_uuid: Uuid,
    characteristics: HashSet<Characteristic>,
    peripheral_name: String,
) -> Result<(), RustlerError> {
    println!("[Rust] Adding service to peripheral...");

    {
        let peripheral_guard = peripheral.write().await;
        if let Err(e) = peripheral_guard.add_service(&Service::new(service_uuid, true, characteristics)) {
            println!("[Rust] ❌ Failed to add service: {:?}", e);
            return Err(RustlerError::Atom("failed_to_add_service"));
        }
    }

    println!("[Rust] Service added successfully. Starting advertising...");

    {
    let peripheral_guard = peripheral.write().await;
    println!("[Rust] Attempting to start advertising in discoverable mode...");
    
    match peripheral_guard.start_advertising(&peripheral_name, &[service_uuid]).await {
        Ok(_) => println!("[Rust] ✅ Advertising started successfully in discoverable mode!"),
        Err(e) => {
            println!("[Rust] ❌ Failed to start advertising: {:?}", e);
            return Err(RustlerError::Atom("failed_to_start_advertising"));
        }
    }
}

    println!("[Rust] Peripheral started advertising");
    Ok(())
}

async fn stop_advertising(state: Arc<Mutex<GattPeripheralState>>) -> Result<(), RustlerError> {
    println!("[Rust] Stopping advertising...");

    let peripheral = {
        let state_guard = state.lock().unwrap();
        state_guard.peripheral.clone()
    };

    {
        let peripheral_guard = peripheral.write().await;
        if let Err(e) = peripheral_guard.stop_advertising().await {
            println!("[Rust] Failed to stop advertising: {:?}", e);
            return Err(RustlerError::Atom("failed_to_stop_advertising"));
        }
    }

    println!("[Rust] Peripheral stopped advertising");
    Ok(())
}

fn create_characteristics(
    sender_characteristic: futures::channel::mpsc::Sender<Event>,
    sender_descriptor: futures::channel::mpsc::Sender<Event>,
) -> HashSet<Characteristic> {
    let mut characteristics = HashSet::new();
    let char_uuid = short_uuid_to_uuid(0x2A3D);

    let properties = characteristic::Properties::new(
        Some(characteristic::Read(characteristic::Secure::Insecure(sender_characteristic.clone()))),
        Some(characteristic::Write::WithResponse(characteristic::Secure::Insecure(sender_characteristic.clone()))),
        Some(sender_characteristic),
        None,
    );

    let mut descriptors = HashSet::new();
    let desc_uuid = short_uuid_to_uuid(0x2902);
    descriptors.insert(Descriptor::new(
        desc_uuid,
        descriptor::Properties::new(
            Some(descriptor::Read(descriptor::Secure::Insecure(sender_descriptor.clone()))),
            Some(descriptor::Write(descriptor::Secure::Insecure(sender_descriptor))),
        ),
        None,
    ));

    characteristics.insert(Characteristic::new(
        char_uuid,
        properties,
        None,
        descriptors,
    ));

    characteristics
}


async fn handle_characteristics(mut receiver: futures::channel::mpsc::Receiver<Event>) {
    let characteristic_value = Arc::new(RwLock::new(String::from("Initial Value")));
    let notifying = Arc::new(atomic::AtomicBool::new(false));

    while let Some(event) = receiver.next().await {
        match event {
            Event::ReadRequest(request) => {
                let value = characteristic_value.read().await.clone();
                request.response.send(Response::Success(value.into_bytes())).unwrap();
            }
            Event::WriteRequest(request) => {
                if let Ok(value) = String::from_utf8(request.data) {
                    let mut characteristic_value = characteristic_value.write().await;
                    *characteristic_value = value;
                }
                request.response.send(Response::Success(vec![])).unwrap();
            }
            Event::NotifySubscribe(mut subscribe) => {
                notifying.store(true, atomic::Ordering::SeqCst);
                let notify_characteristic = characteristic_value.clone();
                let notify_flag = notifying.clone();

                tokio::spawn(async move {
                    let mut counter = 0;
                    while notify_flag.load(atomic::Ordering::SeqCst) {
                        counter += 1;
                        let value = format!("Notification {}", counter);
                        subscribe.notification.try_send(value.into_bytes()).unwrap_or_default();
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                });
            }
            Event::NotifyUnsubscribe => {
                notifying.store(false, atomic::Ordering::SeqCst);
            }
        }
    }
}

async fn handle_descriptors(mut receiver: futures::channel::mpsc::Receiver<Event>) {
    let descriptor_value = Arc::new(RwLock::new(vec![0u8; 2]));

    while let Some(event) = receiver.next().await {
        match event {
            Event::ReadRequest(request) => {
                let value = descriptor_value.read().await.clone();
                request.response.send(Response::Success(value)).unwrap();
            }
            Event::WriteRequest(request) => {
                let mut descriptor_value = descriptor_value.write().await;
                *descriptor_value = request.data;
                request.response.send(Response::Success(vec![])).unwrap();
            }
            _ => {}
        }
    }
}


fn short_uuid_to_uuid(short_uuid: u16) -> Uuid {
    let mut bytes = [0x00; 16];
    bytes[6] = 0x10;
    bytes[8] = 0x80;
    bytes[9] = 0x00;
    bytes[12] = (short_uuid >> 8) as u8;
    bytes[13] = (short_uuid & 0xFF) as u8;
    Uuid::from_bytes(bytes)
}
