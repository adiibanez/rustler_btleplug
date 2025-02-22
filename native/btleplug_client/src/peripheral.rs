use crate::atoms;

use log::{debug, error, info, warn};
use pretty_env_logger;

use crate::RUNTIME;
use futures::StreamExt;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc, Term};
use std::sync::{Arc, Mutex};

// use btleplug::api::Peripheral;
use btleplug::api::Peripheral as ApiPeripheral;
use btleplug::platform::Peripheral;

//use bluster::gatt::characteristic::Characteristic;
use btleplug::api::Characteristic;

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
        debug!("PeripheralResource destructor called.");
    }
}

#[rustler::nif]
pub fn connect(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let resource_arc = resource.0.clone();

    let pid = env.pid();

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        let peripheral = {
            let peripheral_state = resource_arc.lock().unwrap();
            peripheral_state.peripheral.clone()
        };

        debug!("Connecting to Peripheral: {:?}", peripheral.id());

        if let Err(e) = peripheral.connect().await {
            warn!("Failed to connect: {:?}", e);
        } else {
            info!("Successfully connected to peripheral.");

            match msg_env.send_and_clear(&pid, |env| {
                (
                    atoms::btleplug_device_connected(),
                    peripheral.id().to_string(),
                )
                    .encode(env)
            }) {
                Ok(_) => {
                    debug!("Successfully sent characteristic value changed message")
                }
                Err(e) => debug!(
                    "Failed to send sent characteristic value changed message (Error: {:?}). \
                    This might happen if the Elixir process has terminated.",
                    e
                ),
            }

            // Discover services after successful connection
            if let Err(e) = peripheral.discover_services().await {
                warn!("Failed to discover services: {:?}", e);

                match msg_env.send_and_clear(&pid, |env| {
                    (
                        atoms::btleplug_device_service_discovery_error(),
                        peripheral.id().to_string(),
                    )
                        .encode(env)
                }) {
                    Ok(_) => {
                        debug!("Successfully sent characteristic value changed message")
                    }
                    Err(e) => debug!(
                        "Failed to send sent characteristic value changed message (Error: {:?}). \
                    This might happen if the Elixir process has terminated.",
                        e
                    ),
                }
            } else {
                debug!("Services discovered successfully.");
            }
        }
    });

    Ok(resource)
}

// async fn get_characteristics(peripheral: &Peripheral) -> Result<Vec<Characteristic>, String> {
//     // ✅ Cast `peripheral` into a reference of `ApiPeripheral` to call the method
//     let characteristics = ApiPeripheral::discover_characteristics(peripheral.as_ref())
//         .await
//         .map_err(|e| format!("Discover characteristics error: {:?}", e))?;

//     info!("Characteristics length: {}", characteristics.len());

//     for characteristic in &characteristics {
//         println!("Characteristic: {:?}", characteristic);
//     }

//     Ok(characteristics)
// }

#[rustler::nif]
pub fn subscribe(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
    characteristic_uuid: String,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let resource_arc = resource.0.clone();
    let pid = env.pid();

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        let peripheral = {
            let peripheral_state = resource_arc.lock().unwrap();
            peripheral_state.peripheral.clone()
        };

        // ✅ Call `discover_characteristics()` correctly
        // let characteristics = ApiPeripheral::discover_characteristics(&peripheral)
        //     .await
        //     .map_err(|e| format!("Discover characteristics error: {:?}", e))?;

        // ✅ Use the instance method instead of calling it on the trait

        match peripheral.discover_services().await {
            Ok(services) => {
                info!("Services length: {:?}", services);
            }
            Err(e) => {
                 warn!("Error discovering services {:?}", e);
            }
        };
        let characteristics = peripheral.characteristics();
            
    // .map_err(|e| format!("Discover characteristics error: {:?}", e))?;

        info!("Characteristics length: {:?}", characteristics.len());

        for characteristic in &characteristics {
            info!("Characteristic: {:?}", characteristic);
        }

        let characteristic = characteristics
            .iter()
            .find(|c| c.uuid.to_string() == characteristic_uuid)
            .cloned();

        match characteristic {
            Some(char) => {
                info!("Subscribing to characteristic: {:?}", char.uuid);

                match peripheral.subscribe(&char).await {
                    Ok(_) => {
                        match peripheral.notifications().await {
                            Ok(mut notifications) => {
                                while let Some(notification) = notifications.next().await {
                                    debug!("Received Notification: {:?}", notification.value);

                                    match msg_env.send_and_clear(&pid, |env| {
                                        (atoms::btleplug_characteristic_value_changed(), char.uuid.to_string()).encode(env)
                                    }) {
                                        Ok(_) => {
                                            debug!("Successfully sent characteristic value changed message")
                                        }
                                        Err(e) => debug!(
                                            "Failed to send characteristic value changed message (Error: {:?}). \
                                This might happen if the Elixir process has terminated.",
                                            e
                                        ),
                                    }
                                }
                            }
                            Err(e) => warn!("Failed to get notifications: {:?}", e),
                        }
                    }
                    Err(e) => warn!("Failed to subscribe: {:?}", e),
                }
            }
            None => info!("Characteristic not found: {}", characteristic_uuid),
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
