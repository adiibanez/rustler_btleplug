use crate::atoms;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, ResourceArc, Term};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use tokio::runtime::Runtime;
use tokio::spawn;

pub struct PeripheralRef(pub(crate) Arc<Mutex<PeripheralState>>);

pub struct PeripheralState {
    pub pid: LocalPid,
    pub peripheral: Peripheral,
}

impl PeripheralState {
    pub fn new(pid: LocalPid, peripheral: Peripheral) -> Self {
        PeripheralState { pid, peripheral }
    }
}

impl Drop for PeripheralState {
    fn drop(&mut self) {
        println!("[Rust] PeripheralResource destructor called.");
    }
}

pub fn load(env: Env) -> bool {
    rustler::resource!(PeripheralRef, env);
    true
}

#[rustler::nif]
pub fn connect(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let resource_arc = resource.0.clone();

    spawn(async move {
        let mut peripheral_state = resource_arc.lock().unwrap();
        let peripheral = peripheral_state.peripheral;

        println!("[Rust] Connecting to Peripheral: {:?}", peripheral.id());

        if let Err(e) = peripheral.connect().await {
            println!("[Rust] Failed to connect: {:?}", e);
        } else {
            println!("[Rust] Successfully connected to peripheral.");
        }
    });

    Ok(resource)
}

#[rustler::nif]
pub fn subscribe(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
    characteristic_uuid: String,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let resource_arc = resource.0.clone();

    spawn(async move {
        let mut peripheral_state = resource_arc.lock().unwrap();
        let peripheral = peripheral_state.peripheral;

        let characteristics = peripheral
            .characteristics()
            .await
            .expect("Failed to get characteristics");
        let characteristic = characteristics
            .iter()
            .find(|c| c.uuid.to_string() == characteristic_uuid)
            .cloned();

        if let Some(char) = characteristic {
            println!("[Rust] Subscribing to characteristic: {:?}", char.uuid);

            if let Err(e) = peripheral.subscribe(&char).await {
                println!("[Rust] Failed to subscribe: {:?}", e);
                return;
            }

            let mut notifications = peripheral.notifications().await.unwrap();
            while let Some(notification) = notifications.next().await {
                //env.send_message(("btleplug_notification".encode(env), notification.value));
                /*env.send_and_clear(&peripheral.pid, |env| {
                    ("btleplug_notification".encode(env), notification.value).encode(env)
                });*/
            }
        } else {
            println!("[Rust] Characteristic not found: {}", characteristic_uuid);
        }
    });

    Ok(resource)
}
