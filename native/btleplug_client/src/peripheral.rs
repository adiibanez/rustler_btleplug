use crate::atoms;
use crate::RUNTIME;
use futures::StreamExt;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, ResourceArc, Term};
use std::sync::{Arc, Mutex};

use btleplug::api::Peripheral as _;
use btleplug::platform::Peripheral;

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

    RUNTIME.spawn(async move {
        let peripheral = {
            let peripheral_state = resource_arc.lock().unwrap();
            peripheral_state.peripheral.clone()
        };

        println!("[Rust] Connecting to Peripheral: {:?}", peripheral.id());

        if let Err(e) = peripheral.connect().await {
            println!("[Rust] Failed to connect: {:?}", e);
        } else {
            println!("[Rust] Successfully connected to peripheral.");

            // Discover services after successful connection
            if let Err(e) = peripheral.discover_services().await {
                println!("[Rust] Failed to discover services: {:?}", e);
            } else {
                println!("[Rust] Services discovered successfully.");
            }
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

    RUNTIME.spawn(async move {
        let peripheral = {
            let peripheral_state = resource_arc.lock().unwrap();
            peripheral_state.peripheral.clone()
        };

        let characteristics = peripheral.characteristics();

        for characteristic in &characteristics {
            println!("[Rust] Characteristic: {:?}", characteristic);
        }

        let characteristic = characteristics
            .iter()
            .find(|c| c.uuid.to_string() == characteristic_uuid)
            .cloned();

        match characteristic {
            Some(char) => {
                println!("[Rust] Subscribing to characteristic: {:?}", char.uuid);

                match peripheral.subscribe(&char).await {
                    Ok(_) => {
                        match peripheral.notifications().await {
                            Ok(mut notifications) => {
                                while let Some(notification) = notifications.next().await {
                                    println!(
                                        "[Rust] Received Notification: {:?}",
                                        notification.value
                                    );
                                    // Here you could send notifications back to Elixir using the pid
                                }
                            }
                            Err(e) => println!("[Rust] Failed to get notifications: {:?}", e),
                        }
                    }
                    Err(e) => println!("[Rust] Failed to subscribe: {:?}", e),
                }
            }
            None => println!("[Rust] Characteristic not found: {}", characteristic_uuid),
        }
    });

    Ok(resource)
}

// // Optional: Add a function to handle sending notifications back to Elixir
// fn send_notification_to_elixir(pid: &LocalPid, value: Vec<u8>) {
//     let mut msg_env = rustler::env::OwnedEnv::new();
//     msg_env.send_and_clear(pid, |env| {
//         (atoms::notification(), value).encode(env)
//     }).unwrap();
// }
