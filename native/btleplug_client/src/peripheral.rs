use crate::atoms;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, ResourceArc, Term};
use std::sync::{Arc, Mutex};
use tokio::spawn;
use futures::StreamExt; // ✅ Fix: Import StreamExt

use btleplug::api::Peripheral as _; // ✅ Fix: Import Peripheral trait methods
use btleplug::platform::Peripheral;

pub fn load(env: Env) -> bool {
    rustler::resource!(PeripheralRef, env);
    rustler::resource!(PeripheralState, env);
    true
}

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


#[rustler::nif]
pub fn connect(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let resource_arc = resource.0.clone();

    spawn(async move {
        let peripheral = {
            let peripheral_state = resource_arc.lock().unwrap();
            peripheral_state.peripheral.clone()
        };

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
        let peripheral = {
            let peripheral_state = resource_arc.lock().unwrap();
            peripheral_state.peripheral.clone()
        };

        let characteristics = peripheral.characteristics(); // ✅ Fix: Removed `.await`

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

            let mut notifications = match peripheral.notifications().await {
                Ok(n) => n,
                Err(_) => {
                    println!("[Rust] Failed to get notifications");
                    return;
                }
            };

            while let Some(notification) = notifications.next().await { // ✅ Fix: `.next()` now works
                println!("[Rust] Received Notification: {:?}", notification.value);
            }
        } else {
            println!("[Rust] Characteristic not found: {}", characteristic_uuid);
        }
    });

    Ok(resource)
}
